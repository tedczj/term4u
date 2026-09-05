use itertools::Itertools as _;
use warp_util::path::ShellFamily;

pub mod model;

pub use model::{EnvVar, EnvVarCollection, EnvVarCommand, EnvVarValue};

use crate::terminal::shell::ShellType;

pub trait EnvVarExt {
    fn get_initialization_string(&self, shell_type: ShellType) -> String;
}

impl EnvVarExt for EnvVar {
    fn get_initialization_string(&self, shell_type: ShellType) -> String {
        let shell_family = ShellFamily::from(shell_type);
        let name = shell_family.escape(&self.name);
        let value = initialization_value(&self.value, shell_family);
        match shell_type {
            ShellType::Bash | ShellType::Zsh => format!("export {name}={value};"),
            ShellType::Fish => format!("set -x {name} {value};"),
            ShellType::PowerShell => format!("$env:{name} = {value};"),
        }
    }
}

pub trait EnvVarCollectionExt {
    fn export_variables_for_shell(&self, shell_type: ShellType) -> String;
}

impl EnvVarCollectionExt for EnvVarCollection {
    fn export_variables_for_shell(&self, shell_type: ShellType) -> String {
        serialize_variables_for_shell(
            self.vars.iter().map(|variable| (variable.name.as_str(), &variable.value)),
            shell_type,
        )
    }
}

pub fn serialize_variables_for_shell<'a>(
    pairs: impl IntoIterator<Item = (&'a str, &'a EnvVarValue)>,
    shell_type: ShellType,
) -> String {
    let shell_family = ShellFamily::from(shell_type);
    let (prefix, separator, postfix) = match shell_type {
        ShellType::Fish => ("set -x ", " ", ";"),
        ShellType::Bash | ShellType::Zsh => ("", "=", ""),
        ShellType::PowerShell => ("$env:", " = ", ";"),
    };
    pairs
        .into_iter()
        .map(|(name, value)| {
            format!(
                "{prefix}{}{separator}{}{postfix}",
                shell_family.escape(name),
                initialization_value(value, shell_family)
            )
        })
        .join(" ")
}

fn initialization_value(value: &EnvVarValue, shell_family: ShellFamily) -> String {
    match value {
        EnvVarValue::Constant(value) => match shell_family {
            ShellFamily::Posix => shell_family.escape(value).into_owned(),
            ShellFamily::PowerShell => format!("'{}'", value.replace('\'', "''")),
        },
        EnvVarValue::Command(command) => format!("$({})", command.command),
    }
}
