use std::fmt;
use std::error::Error;
use futures_util::core_reexport::fmt::Formatter;
use std::convert::TryFrom;
use regex::Regex;

#[derive(Debug)]
pub struct ImageNameError {
    msg: String
}

impl fmt::Display for ImageNameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Error for ImageNameError {}

pub struct DockerImageName<T> {
    host: Option<T>,

    port: Option<u16>,

    name_components: Vec<T>,

    pub tag: Option<T>,
}

impl TryFrom<String> for DockerImageName<String> {
    type Error = ImageNameError;

    fn try_from(image_name: String) -> Result<Self, Self::Error> {
        let components = image_name.split("/").collect::<Vec<&str>>();

        match components.len() {
            1 => {
                extract_name_and_tag(components[0])
                    .map(|x| {
                        DockerImageName {
                            host: Some("registry-1.docker.io".to_string()),
                            port: None,
                            name_components: vec![x.0],
                            tag: x.1
                        }
                    })
            },
            _ => {
                let (host, port) = match components.get(0) {
                    Some(&maybe_host) => {
                        if could_be_hostname(maybe_host) {
                            let parts = maybe_host.split(":").collect::<Vec<&str>>();
                            match parts.len() {
                                2 => ((Some(parts[0].to_owned()), Some(parts[1].parse::<u16>().unwrap()))),
                                _ => ((Some(maybe_host.to_owned()), None)),
                            }
                        } else { (None, None) }
                    },
                    None => (None, None)
                };

                extract_name_and_tag(components.last().unwrap())
                    .map(|x| {
                        DockerImageName {
                            host,
                            port,
                            name_components: components.iter().map(|x| x.to_string()).collect(),
                            tag: x.1
                        }
                    })
            }
        }
    }
}

fn extract_name_and_tag(name_component: &str) -> Result<(String, Option<String>), ImageNameError> {
    let name_and_maybe_tag = name_component.split(":").collect::<Vec<&str>>();

    match name_and_maybe_tag.len() {
        1 => {
            Ok((name_and_maybe_tag[0].to_owned(), None))
        },
        2 => {
            Ok((name_and_maybe_tag[0].to_owned(), Some(name_and_maybe_tag[1].to_owned())))
        },
        _ => {
            Err(ImageNameError { msg: "Too many colons in the last name component".to_string() })
        }
    }
}

fn could_be_hostname(name_component: &str) -> bool {
    let host_regex = Regex::new(r"([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])(\.([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]{0,61}[a-zA-Z0-9]))*").unwrap();

    return host_regex.is_match(name_component);
}

#[cfg(test)]
mod docker_image_name_tests {
    use crate::docker_image_name::DockerImageName;
    use std::convert::TryFrom;
    use std::net::ToSocketAddrs;

    #[test]
    fn image_name() {
        let image_name_result = DockerImageName::try_from("httpd".to_string());

        assert!(image_name_result.is_ok());
        let image_name = image_name_result.unwrap();

        assert_eq!(1, image_name.name_components.len());
        assert_eq!("httpd", &image_name.name_components[0]);
        assert!(image_name.port.is_none());
        assert!(image_name.tag.is_none());
        assert_eq!(Some("registry-1.docker.io".to_string()), image_name.host);
    }

    #[test]
    fn owned_image_name_with_version() {
        let image_name = DockerImageName::try_from("fedora/httpd:version1.0".to_string());

        let docker_image_name = image_name.unwrap();
        assert_eq!(2, docker_image_name.name_components.len());
        assert!(docker_image_name.host.is_some());
        assert_eq!("fedora", docker_image_name.host.unwrap());
        assert!(docker_image_name.port.is_none());
        assert!(docker_image_name.tag.is_some());
        assert_eq!("version1.0", docker_image_name.tag.unwrap());
    }

    #[test]
    fn owned_image_name_with_test_version() {
        let image_name = DockerImageName::try_from("fedora/httpd:version1.0.test".to_string());

        let docker_image_name = image_name.unwrap();
        assert_eq!(2, docker_image_name.name_components.len());
        assert!(docker_image_name.host.is_some());
        assert_eq!("fedora", docker_image_name.host.unwrap());
        assert!(docker_image_name.port.is_none());
        assert!(docker_image_name.tag.is_some());
        assert_eq!("version1.0.test", docker_image_name.tag.unwrap());
    }

    #[test]
    fn owned_image_name_with_registry() {
        let image_name = DockerImageName::try_from("myregistryhost:5000/fedora/httpd:version1.0".to_string());

        let docker_image_name = image_name.unwrap();
        assert_eq!(3, docker_image_name.name_components.len());
        assert!(docker_image_name.host.is_some());
        assert_eq!("myregistryhost", docker_image_name.host.unwrap());
        assert!(docker_image_name.port.is_some());
        assert_eq!(5000, docker_image_name.port.unwrap());
        assert!(docker_image_name.tag.is_some());
        assert_eq!("version1.0", docker_image_name.tag.unwrap());
    }
}
