use std::{fs::File, io::Read, net::IpAddr};

use quick_xml::{
    events::{BytesStart, Event, attributes::Attributes},
    name::QName,
};
use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

/// Macro pro parsovani s pekne formatovanou pripadnou chybou
macro_rules! parse {
    ($i: expr, $t: ty) => {
        $i.parse::<$t>().map_err(|e| ConfigError::ParseError {
            input: $i.to_string(),
            to: stringify!($t).to_string(),
            cause: e.to_string(),
        })
    };
}

/// Structura pro nactena data
#[derive(Debug)]
pub struct XmlData {
    pub rjss: Vec<Rjs>,
    pub diagnet: Vec<DlsIP>,
}

/// Data RJS
#[derive(Debug)]
pub struct Rjs {
    prejezd: String,
    ip: IpAddr,
    port: u16,
    rjs_type: String,
}

/// Data DlsIP
#[derive(Debug)]
pub enum DlsIP {
    Hosts {
        alias: String,
        connection: Connection,
    },
    Static {
        ip: IpAddr,
    },
}

/// DlsIP connection type
#[derive(Debug)]
pub enum Connection {
    P2P,
    Broadcast,
}

impl DlsIP {
    /// Vytvori DlsIP z xml atributu
    fn from_attributes(attrs: Attributes) -> Result<Self> {
        let source = find_attribute(&attrs, "source")?;

        match source.as_str() {
            "HOSTS" => {
                let connection = match find_attribute(&attrs, "connection")?.as_str() {
                    "P2P" => Connection::P2P,
                    "BROADCAST" => Connection::Broadcast,
                    i => {
                        return Err(ConfigError::InvalidValue {
                            name: "connection".into(),
                            value: i.to_string(),
                        }
                        .into());
                    }
                };

                Ok(DlsIP::Hosts {
                    alias: find_attribute(&attrs, "alias")?,
                    connection,
                })
            }
            "STATIC" => {
                let ip = find_attribute(&attrs, "ip")?;

                Ok(DlsIP::Static {
                    ip: parse!(ip, IpAddr)?,
                })
            }
            i => Err(ConfigError::InvalidValue {
                name: "source".into(),
                value: i.to_string(),
            }
            .into()),
        }
    }
}

impl Rjs {
    /// Vytvori RJS z xml atributu
    fn from_attributes(attrs: Attributes) -> Result<Self> {
        let ip = find_attribute(&attrs, "ip")?;
        let port = find_attribute(&attrs, "port")?;

        let rjs = Rjs {
            prejezd: find_attribute(&attrs, "prejezd")?,
            ip: parse!(ip, IpAddr)?,
            port: parse!(port, u16)?,
            rjs_type: find_attribute(&attrs, "type")?,
        };

        Ok(rjs)
    }
}

/// Najde atribut a vrati jeho hodnotu jako string
fn find_attribute<'a>(attrs: &Attributes<'a>, data: &str) -> Result<String> {
    let attr = attrs.clone().find_map(|i| {
        if i.is_ok() && i.as_ref().unwrap().key == QName(data.as_bytes()) {
            Some(i.unwrap())
        } else {
            None
        }
    });

    if let Some(attr) = attr {
        return Ok(String::from_utf8(attr.value.to_vec()).unwrap());
    } else {
        return Err(ConfigError::MissingAttribute {
            name: data.to_string(),
        }
        .into());
    }
}

/// Prida location data k pripadne chybe
fn add_location_data<T>(res: &mut Result<T>, path: &[Option<BytesStart>]) {
    if let Err(Error::ConfigError { location, .. }) = res {
        let path_str = path
            .iter()
            .filter_map(|f| f.as_ref())
            .map(|i| String::from_utf8(i.name().0.to_vec()).unwrap())
            .collect::<Vec<_>>()
            .join("/");

        *location = path_str;
    }
}

impl XmlData {
    /// Nacte data z xml pomoci cesty k souboru
    pub fn read_from_xml_file(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;

        // Cteni souboru do bufferu
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;

        Self::read_from_xml(buffer)
    }

    /// Nacte data z xml
    pub fn read_from_xml(xml: String) -> Result<Self> {
        // Vytvoreni prazdnych dat
        let mut data = Self::default();

        // xml reader
        let mut xml = quick_xml::Reader::from_str(&xml);

        // data o aktualni pozici v xml souboru s maximalni hloubkou 10
        let mut depth = 0;
        let mut path: [Option<BytesStart>; _] = [const { None }; 10];

        loop {
            match xml.read_event() {
                Ok(e) => {
                    match e {
                        // Kontroluje aktualni pozici
                        Event::Start(e) => {
                            if path.len() > depth {
                                path[depth] = Some(e);
                            }
                            depth += 1;
                        }
                        Event::End(_) => {
                            depth -= 1;
                            if path.len() > depth {
                                path[depth] = None;
                            }
                        }
                        Event::Empty(e) => {
                            if path.len() > depth {
                                path[depth] = Some(e.clone());
                            }
                            depth += 1;

                            // Kontrola zda byl nalezen RJS / DLSIP a pripadne pridani do dat
                            match e.name() {
                                QName(b"rjs") => {
                                    let mut rjs = Rjs::from_attributes(e.attributes());

                                    add_location_data(&mut rjs, &path);

                                    data.rjss.push(rjs?);
                                }
                                QName(b"dlsip") => {
                                    let mut dlsip = DlsIP::from_attributes(e.attributes());

                                    add_location_data(&mut dlsip, &path);

                                    data.diagnet.push(dlsip?);
                                }
                                _ => {}
                            }

                            depth -= 1;
                        }
                        // Konec souboru
                        Event::Eof => {
                            break;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(data)
    }
}

/// Chyba pri nacitani dat z xml
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),

    #[error("{} at {}", .error, .location)]
    ConfigError {
        error: ConfigError,
        location: String,
    },
}

/// Chyby ktere maji lokaci
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Error parsing {} as {}", .input, .to)]
    ParseError {
        input: String,
        to: String,
        cause: String,
    },

    #[error("Missing {} arribute", .name)]
    MissingAttribute { name: String },

    #[error("Invalid value {} in {} arribute ", .value, .name)]
    InvalidValue { name: String, value: String },
}

/// Prevod ConfigError na Error bez lokace
impl From<ConfigError> for Error {
    fn from(value: ConfigError) -> Self {
        Self::ConfigError {
            error: value,
            location: "unknown location".into(),
        }
    }
}

/// Prazdne XmlData
impl Default for XmlData {
    fn default() -> Self {
        Self {
            rjss: Vec::new(),
            diagnet: Vec::new(),
        }
    }
}
