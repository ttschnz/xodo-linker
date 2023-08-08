use dirs::home_dir;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::from_reader;
use std::error::Error;
use std::fs::{canonicalize, File};
use std::path::Path;
use std::process::Command;
use substring::Substring;
use tiny_http::{Header, Request, Response, Server};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Linker {
    pub security: SecurityConfig,
    pub server: ServerConfig,
    pub system: SystemConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemConfig {
    hostname: String, // TODO
    base_path: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        SystemConfig {
            hostname: "xodo".to_string(),
            base_path: r"{{home_dir}}\OneDrive\ONEDRI~1".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerConfig {
    port: u16,

    addr: String,
    close_tab: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: 80,
            addr: "0.0.0.0".to_string(),
            close_tab: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SecurityConfig {
    force_loopback: bool,
    #[serde(with = "serde_regex")]
    blacklist: Vec<Regex>,
    #[serde(with = "serde_regex")]
    whitelist: Vec<Regex>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            force_loopback: true,
            blacklist: vec![
                Regex::new(r"/favicon\.ico").expect("expected preprogrammed regex to be ok")
            ],
            whitelist: vec![Regex::new(r".*\.pdf").expect("expected preprogrammed regex to be ok")],
        }
    }
}
impl SystemConfig {
    pub fn get_absolute_pdf_path(&self, requested_path: &str) -> Result<String, String> {
        match home_dir() {
            Some(home) => {
                println!("getting absolute path for {}", requested_path);
                // join base_path and requested_path
                let file_path = Path::new(&self.base_path.replace(
                    "{{home_dir}}",
                    home.to_str().ok_or("could not resolve home")?,
                ))
                .join(requested_path.substring(1, requested_path.len()));
                // println!("gotten filepath {:?}", file_path);

                // canonicalize the path
                let file_path = canonicalize(file_path)
                    .map_err(|e| format!("Could not canonicalize: {}", e))?;
                // println!("resolved to  {:?}", file_path);

                // remove prefix
                let file_path = file_path
                    .to_str()
                    .ok_or("could not transform path to string")?
                    .to_string();

                if file_path.starts_with(r"\\?\") {
                    println!("removed prefix");
                    Ok(file_path.replacen(r"\\?\", "", 1))
                } else {
                    Ok(file_path)
                }

                // transform to string
            }
            None => Err("did not find home dir".to_string()),
        }
    }

    #[cfg(target_os = "windows")]
    fn open_file_with_dialog(&self, path_to_file: String) -> Result<(), String> {
        // powershell.exe -command "openwith \"path\to\file\with\backslashes.pdf\""

        println!("opening file dialog for file {}", path_to_file);

        let output = Command::new("powershell")
            .args(["-command", &format!("openwith \"{}\"", { path_to_file })])
            .output()
            .map_err(|e| format!("process did not finish successfully: {}", e))?;

        if output.status.success() {
            println!("{:?}", output);
            Ok(())
        } else {
            Err(format!(
                "process failed. stderr: {:?}",
                String::from_utf8(output.stderr).unwrap()
            ))
        }
    }

    #[cfg(target_os = "windows")]
    pub fn run(&self, path: &str) -> Result<(), String> {
        let path_to_file = self.get_absolute_pdf_path(path)?;
        assert_eq!(
            r"C:\Users\tim\OneDrive\OneDrive - epfl.ch\test.pdf",
            path_to_file
        );
        self.open_file_with_dialog(path_to_file)
    }
}

impl SecurityConfig {
    fn matches_blacklist(&self, path: &str) -> bool {
        SecurityConfig::matches_list(&self.blacklist, path)
    }
    fn matches_whitelist(&self, path: &str) -> bool {
        SecurityConfig::matches_list(&self.whitelist, path)
    }
    fn matches_list(list: &Vec<Regex>, path: &str) -> bool {
        list.iter().fold(false, |acc, r| r.is_match(path) || acc)
    }
    fn allow_request(&self, request: &Request) -> bool {
        vec![
            self.allow_force_loopback(request),
            self.allow_black_and_white_list(request),
        ]
        .iter()
        .all(|b| b == &true)
    }
    fn allow_force_loopback(&self, request: &Request) -> bool {
        if self.force_loopback {
            request
                .remote_addr()
                .and_then(|addr| Some(addr.ip().is_loopback()))
                .unwrap_or(false)
        } else {
            true
        }
    }

    fn allow_black_and_white_list(&self, request: &Request) -> bool {
        let url = request.url();
        if self.matches_blacklist(url) {
            if self.matches_whitelist(url) {
                true
            } else {
                false
            }
        } else {
            true
        }
    }
}

impl ServerConfig {
    pub fn get_server(&self) -> Result<Server, Box<dyn Error + Send + Sync + 'static>> {
        Server::http((self.addr.as_str(), self.port))
    }

    pub fn handle_request(&self, request: Request, is_allowed: bool, did_succeed: Option<bool>) {
        let response = if is_allowed {
            if did_succeed.unwrap_or(false) {
                if self.close_tab {
                    let mut response = Response::from_string(
                        "<script>window.close()</script>tab should close now",
                    );
                    response.add_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                    );
                    response
                } else {
                    Response::from_string("ok")
                }
            } else {
                Response::from_string("failed to start. check logs")
            }
        } else {
            Response::from_string("does not comply").with_status_code(401)
        };
        if let Err(err) = request.respond(response) {
            println!("could not respond to request: {}", err);
        }
    }
}

impl Linker {
    pub fn read_config(path: &str) -> Linker {
        match File::open(path) {
            Ok(reader) => match from_reader(reader) {
                Err(err) => {
                    println!(
                        "Warning: using default configuration. Could not parse file: {}",
                        err
                    );
                    Linker::default()
                }
                Ok(cfg) => cfg,
            },
            Err(_) => {
                println!("Warning: using default configuration. Could not find file.");
                Linker::default()
            }
        }
    }

    pub fn allow_request(&self, request: &Request) -> bool {
        self.security.allow_request(request)
    }

    pub fn get_server(&self) -> Result<Server, Box<dyn Error + Send + Sync + 'static>> {
        self.server.get_server()
    }

    pub fn handle_request(&self, request: Request) {
        let is_allowed = self.allow_request(&request);
        let did_succeed = if is_allowed {
            println!("Request passed all security-checks.");
            Some(
                self.system
                    .run(&request.url())
                    .map_err(|e| {
                        println!("failed to start: {}", e);
                        ()
                    })
                    .is_ok(),
            )
        } else {
            None
        };
        self.server.handle_request(request, is_allowed, did_succeed)
    }
    pub fn start(&self) {
        let server = self
            .get_server()
            .expect("expected server to start. Is the port blocked or security to strict?");
        println!("Started server. Listening on port {}", self.server.port);
        for request in server.incoming_requests() {
            println!("Received request.");
            self.handle_request(request)
        }
    }
}

#[cfg(test)]
mod test {
    use super::Linker;

    #[test]
    fn opens_dialog() {
        Linker::default()
            .system
            .open_file_with_dialog(r"C:\Users\tim\Downloads\test.pdf".to_string())
            .unwrap()
    }
    #[test]
    fn opens_dialog_onedrive() {
        Linker::default()
            .system
            // .open_file_with_dialog(r"C:\Users\tim\OneDrive\OneDrive - epfl.ch\test.pdf".to_string())
            .open_file_with_dialog(r"C:\Users\tim\OneDrive\OneDrive - epfl.ch\test.pdf".to_string())
            .unwrap()
    }
}
