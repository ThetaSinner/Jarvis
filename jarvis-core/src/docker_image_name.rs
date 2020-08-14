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
    raw: T,

    host: T,

    port: Option<u16>,

    name_components: Vec<T>,

    tag: Option<T>,
}

impl TryFrom<String> for DockerImageName<String> {
    type Error = ImageNameError;

    fn try_from(image_name: String) -> Result<Self, Self::Error> {
        let components = image_name.split("/").collect::<Vec<&str>>();

        println!("{}", components.len());

        match components.len() {
            1 => {
                extract_name_and_tag(components[0])
                    .map(|x| {
                        DockerImageName {
                            raw: image_name,
                            host: "registry-1.docker.io".to_string(),
                            port: None,
                            name_components: vec![x.0],
                            tag: x.1
                        }
                    })
            },
            _ => {
                try_extract_registry_host(components[0]);
                unimplemented!();
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

fn try_extract_registry_host(name_component: &str) {
    Regex::new(r"");
}

#[cfg(test)]
mod docker_image_name_tests {
    use crate::docker_image_name::DockerImageName;
    use std::convert::TryFrom;

    #[test]
    fn image_name() {
        let image_name_result = DockerImageName::try_from("httpd".to_string());

        assert!(image_name_result.is_ok());
        let image_name = image_name_result.unwrap();

        assert_eq!(1, image_name.name_components.len());
        assert_eq!("httpd", &image_name.name_components[0]);
        assert!(image_name.port.is_none());
        assert!(image_name.tag.is_none());
        assert_eq!("registry-1.docker.io", &image_name.host);
    }

    #[test]
    fn owned_image_name_with_version() {
        let image_name = DockerImageName::try_from("fedora/httpd:version1.0".to_string());

        assert_eq!(1, image_name.unwrap().name_components.len());
    }

    #[test]
    fn owned_image_name_with_test_version() {
        let image_name = DockerImageName::try_from("fedora/httpd:version1.0.test".to_string());

        assert_eq!(1, image_name.unwrap().name_components.len());
    }

    #[test]
    fn owned_image_name_with_registry() {
        let image_name = DockerImageName::try_from("myregistryhost:5000/fedora/httpd:version1.0".to_string());

        assert_eq!(1, image_name.unwrap().name_components.len());
    }
}
