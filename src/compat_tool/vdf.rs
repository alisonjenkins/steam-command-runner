/// Generate compatibilitytool.vdf content
pub fn generate_compatibilitytool_vdf(name: &str, display_name: &str) -> String {
    format!(
        r#""compatibilitytools"
{{
  "compat_tools"
  {{
    "{name}"
    {{
      "install_path" "."
      "display_name" "{display_name}"
      "from_oslist" "windows"
      "to_oslist" "linux"
    }}
  }}
}}
"#,
        name = name,
        display_name = display_name
    )
}

/// Generate toolmanifest.vdf content
pub fn generate_toolmanifest_vdf(require_proton_appid: Option<&str>) -> String {
    let require_line = match require_proton_appid {
        Some(appid) => format!("  \"require_tool_appid\" \"{}\"\n", appid),
        None => String::new(),
    };

    format!(
        r#""manifest"
{{
  "version" "2"
  "commandline" "/steam-command-runner compat %verb%"
{require_line}  "use_sessions" "1"
}}
"#,
        require_line = require_line
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_compatibilitytool_vdf() {
        let vdf = generate_compatibilitytool_vdf("my-tool", "My Tool");
        assert!(vdf.contains("\"my-tool\""));
        assert!(vdf.contains("\"display_name\" \"My Tool\""));
        assert!(vdf.contains("\"from_oslist\" \"windows\""));
        assert!(vdf.contains("\"to_oslist\" \"linux\""));
    }

    #[test]
    fn test_generate_toolmanifest_vdf_without_proton() {
        let vdf = generate_toolmanifest_vdf(None);
        assert!(vdf.contains("\"version\" \"2\""));
        assert!(vdf.contains("/steam-command-runner compat %verb%"));
        assert!(!vdf.contains("require_tool_appid"));
    }

    #[test]
    fn test_generate_toolmanifest_vdf_with_proton() {
        let vdf = generate_toolmanifest_vdf(Some("1493710"));
        assert!(vdf.contains("\"require_tool_appid\" \"1493710\""));
    }
}
