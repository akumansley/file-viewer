use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: String,
    pub template: String,
}

impl FromStr for CommandSpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err("expected <name>: <template>".into());
        }
        let name = parts[0].trim();
        let template = parts[1].trim();
        if name.is_empty() || template.is_empty() {
            return Err("name or template empty".into());
        }
        Ok(CommandSpec {
            name: name.to_string(),
            template: template.to_string(),
        })
    }
}
