/// Configuration for a single service from the Procfile.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub command: String,
}

/// Configuration containing all services from the Procfile.
#[derive(Debug, Clone)]
pub struct Config {
    pub services: Vec<ServiceConfig>,
}

impl Config {
    /// Parses Procfile text into a Config.
    ///
    /// Each line must follow:
    ///     name: command
    pub fn from_str(input: &str) -> Result<Self, String> {
        let mut services = Vec::new();

        for (line_number, line) in input.lines().enumerate() {
            let line = line.trim();

            // Ignore empty lines and comments.
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Split only at the first ':' so colons inside commands are preserved.
            let Some((name, command)) = line.split_once(':') else {
                return Err(format!("Invalid configuration on line {}", line_number + 1));
            };

            let name = name.trim();
            let command = command.trim();

            // A service must have a name.
            if name.is_empty() {
                return Err(format!("Empty service name on line {}", line_number + 1));
            }

            // A service must have a command to execute.
            if command.is_empty() {
                return Err(format!(
                    "Empty command for service '{}' on line {}",
                    name,
                    line_number + 1
                ));
            }

            // Add the valid service to the configuration.
            services.push(ServiceConfig {
                name: name.to_string(),
                command: command.to_string(),
            });
        }

        // A Procfile without any services is invalid.
        if services.is_empty() {
            return Err("No services found".to_string());
        }

        Ok(Self { services })
    }
}

#[test]
fn trims_service_name_and_command() {
    let input = "  gateway  :  npm run gateway  ";

    let config = Config::from_str(input).unwrap();

    assert_eq!(config.services[0].name, "gateway");
    assert_eq!(config.services[0].command, "npm run gateway");
}

#[test]
fn rejects_empty_service_name() {
    let input = ": npm run gateway";

    assert!(Config::from_str(input).is_err());
}

#[test]
fn rejects_empty_command() {
    let input = "gateway:";

    assert!(Config::from_str(input).is_err());
}

#[test]
fn rejects_procfile_with_no_services() {
    let input = r#"
        # only comments

        # gateway
    "#;

    assert!(Config::from_str(input).is_err());
}

#[test]
fn supports_multiple_services() {
    let input = r#"
        api: cargo run --bin api
        frontend: npm run dev
        database: docker compose up db
    "#;

    let config = Config::from_str(input).unwrap();

    assert_eq!(config.services.len(), 3);
    assert_eq!(config.services[1].name, "frontend");
}
