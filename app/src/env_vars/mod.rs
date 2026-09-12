use itertools::Itertools as _;
use warp_util::path::ShellFamily;

pub mod model;

pub use model::EnvVarValue;

use crate::terminal::shell::ShellType;

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
