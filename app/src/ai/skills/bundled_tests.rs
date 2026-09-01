use super::*;

#[test]
fn unavailable_bundled_context_path_renders_as_empty_string() {
    assert_eq!(display_optional_path(None), "");
}

#[test]
fn factory_mcp_bundled_skill_bootstraps_canonical_mcp_resource() {
    let skill = include_str!("../../../../resources/bundled/skills/factory-mcp/SKILL.md");

    assert!(skill.contains("skill://warp/factory-mcp/SKILL.md"));
    assert!(!skill.contains("references/factory-mcp-tools.md"));
}

/// The Factory files skill is always bundled, so a stale trigger description
/// or a broken reference silently reaches every GUI, TUI, and Oz agent. Its
/// trigger has to stay anchored to a factory.yaml root: `agents/<name>/agent.md`
/// alone also describes unrelated agent-definition files.
#[test]
fn factory_files_bundled_skill_is_always_active_and_scoped_to_authoring() {
    let skills_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../resources/bundled/skills")
        .canonicalize()
        .expect("bundled skills directory");
    let skill_dir = skills_dir.join("factory-files");
    let skill = parse_bundled_skill(&skill_dir.join("SKILL.md")).expect("factory-files parses");

    assert_eq!(skill.name, "factory-files");
    let description = skill.description.to_lowercase();
    for intent in ["create", "edit", "factory.yaml", "runner", "scorer"] {
        assert!(
            description.contains(intent),
            "trigger description should mention {intent}: {description}"
        );
    }
    assert!(
        description.contains("factory mcp"),
        "trigger description should exclude Factory MCP operation: {description}"
    );
    assert!(
        description.contains("rooted at a factory.yaml"),
        "trigger description should anchor to a factory.yaml root: {description}"
    );
    assert!(
        description.contains("belongs to another tool"),
        "trigger description should exclude other tools' agent files: {description}"
    );
    assert!(
        skill.content.contains("no `factory.yaml`"),
        "SKILL.md should tell the agent to stop outside a Factory tree"
    );

    assert!(matches!(
        activation_for_bundled_skill("factory-files", &skills_dir),
        BundledSkillActivation::Always
    ));

    for reference in [
        "references/scorers.md",
        "references/examples.md",
        "references/validation.md",
        "scripts/validate_factory_files.py",
    ] {
        assert!(
            skill.content.contains(reference),
            "SKILL.md should point at {reference}"
        );
        assert!(
            skill_dir.join(reference).is_file(),
            "{reference} should exist"
        );
    }
}

/// The skill must not carry a copy of the Factory file format.
///
/// A bundled copy ships inside a Warp release and goes stale against the
/// warp-server it is used against. A stale copy does not fail quietly: it
/// reports fields the server accepts as unknown, and an agent clearing that
/// diagnostic deletes working configuration. An earlier revision did exactly
/// that to the Linear and Slack trigger aliases. The format is fetched from
/// the server now, so nothing here should describe it.
#[test]
fn factory_files_skill_carries_no_copy_of_the_format() {
    let skill_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../resources/bundled/skills/factory-files")
        .canonicalize()
        .expect("factory-files skill directory");

    let mut schemas = Vec::new();
    let mut pending = vec![skill_dir.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("read skill directory") {
            let path = entry.expect("read skill entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.to_string_lossy().ends_with(".schema.json") {
                schemas.push(path);
            }
        }
    }
    assert!(
        schemas.is_empty(),
        "the skill has regrown bundled schemas, which go stale against the server \
         and produce false rejections; fetch the format instead: {schemas:?}"
    );

    let validator = std::fs::read_to_string(skill_dir.join("scripts/validate_factory_files.py"))
        .expect("read the validator");
    for banned in ["import yaml", "def load_yaml", "jsonschema"] {
        assert!(
            !validator.contains(banned),
            "the validator parses the format again ({banned}); it should send bytes \
             to the server and relay the verdict"
        );
    }
    assert!(
        validator.contains("/api/v1/factory-files/validate"),
        "the validator should reach the server's validation endpoint"
    );
}
