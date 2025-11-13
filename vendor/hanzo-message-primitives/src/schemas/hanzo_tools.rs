use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DynamicToolType {
    DenoDynamic,
    PythonDynamic,
    DockerDynamic,
    AgentDynamic,
    McpServerDynamic,
}

impl std::fmt::Display for DynamicToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicToolType::DenoDynamic => write!(f, "deno_dynamic"),
            DynamicToolType::PythonDynamic => write!(f, "python_dynamic"),
            DynamicToolType::DockerDynamic => write!(f, "docker_dynamic"),
            DynamicToolType::AgentDynamic => write!(f, "agent_dynamic"),
            DynamicToolType::McpServerDynamic => write!(f, "mcp_server_dynamic"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CodeLanguage {
    // Primary languages
    #[serde(alias = "Typescript", alias = "TYPESCRIPT")]
    Typescript,
    #[serde(alias = "Python", alias = "PYTHON")]
    Python,
    #[serde(alias = "Javascript", alias = "JAVASCRIPT")]
    Javascript,
    #[serde(alias = "Java", alias = "JAVA")]
    Java,
    #[serde(alias = "Rust", alias = "RUST")]
    Rust,
    #[serde(alias = "Go", alias = "GO")]
    Go,
    #[serde(alias = "Cpp", alias = "CPP", alias = "C++")]
    Cpp,
    #[serde(alias = "C", alias = "c")]
    C,
    #[serde(alias = "Csharp", alias = "CSHARP", alias = "C#")]
    Csharp,
    #[serde(alias = "Ruby", alias = "RUBY")]
    Ruby,
    #[serde(alias = "Php", alias = "PHP")]
    Php,
    #[serde(alias = "Swift", alias = "SWIFT")]
    Swift,
    #[serde(alias = "Kotlin", alias = "KOTLIN")]
    Kotlin,
    #[serde(alias = "Scala", alias = "SCALA")]
    Scala,
    #[serde(alias = "Shell", alias = "SHELL", alias = "Bash", alias = "BASH")]
    Shell,

    // Web technologies
    #[serde(alias = "Html", alias = "HTML")]
    Html,
    #[serde(alias = "Css", alias = "CSS")]
    Css,
    #[serde(alias = "Jsx", alias = "JSX")]
    Jsx,
    #[serde(alias = "Tsx", alias = "TSX")]
    Tsx,

    // Functional languages
    #[serde(alias = "Haskell", alias = "HASKELL")]
    Haskell,
    #[serde(alias = "Elixir", alias = "ELIXIR")]
    Elixir,
    #[serde(alias = "Erlang", alias = "ERLANG")]
    Erlang,
    #[serde(alias = "Clojure", alias = "CLOJURE")]
    Clojure,
    #[serde(alias = "Fsharp", alias = "FSHARP", alias = "F#")]
    Fsharp,
    #[serde(alias = "Ocaml", alias = "OCAML")]
    Ocaml,
    #[serde(alias = "Lisp", alias = "LISP")]
    Lisp,
    #[serde(alias = "Scheme", alias = "SCHEME")]
    Scheme,

    // Data/ML languages
    #[serde(alias = "R", alias = "r")]
    R,
    #[serde(alias = "Julia", alias = "JULIA")]
    Julia,
    #[serde(alias = "Matlab", alias = "MATLAB")]
    Matlab,
    #[serde(alias = "Octave", alias = "OCTAVE")]
    Octave,

    // Database languages
    #[serde(alias = "Sql", alias = "SQL")]
    Sql,
    #[serde(alias = "Plsql", alias = "PLSQL")]
    Plsql,

    // Systems languages
    #[serde(alias = "Zig", alias = "ZIG")]
    Zig,
    #[serde(alias = "Nim", alias = "NIM")]
    Nim,
    #[serde(alias = "Crystal", alias = "CRYSTAL")]
    Crystal,
    #[serde(alias = "D", alias = "d")]
    D,

    // Scripting languages
    #[serde(alias = "Perl", alias = "PERL")]
    Perl,
    #[serde(alias = "Lua", alias = "LUA")]
    Lua,
    #[serde(alias = "Groovy", alias = "GROOVY")]
    Groovy,
    #[serde(alias = "Powershell", alias = "POWERSHELL")]
    Powershell,

    // Mobile languages
    #[serde(alias = "Dart", alias = "DART")]
    Dart,
    #[serde(alias = "Objectivec", alias = "OBJECTIVEC", alias = "Objective-C")]
    Objectivec,

    // Other languages
    #[serde(alias = "Fortran", alias = "FORTRAN")]
    Fortran,
    #[serde(alias = "Cobol", alias = "COBOL")]
    Cobol,
    #[serde(alias = "Pascal", alias = "PASCAL")]
    Pascal,
    #[serde(alias = "Ada", alias = "ADA")]
    Ada,
    #[serde(alias = "Vb", alias = "VB", alias = "VisualBasic")]
    Vb,
    #[serde(alias = "Assembly", alias = "ASSEMBLY", alias = "Asm")]
    Assembly,
    #[serde(alias = "Prolog", alias = "PROLOG")]
    Prolog,

    // Configuration/markup languages
    #[serde(alias = "Yaml", alias = "YAML")]
    Yaml,
    #[serde(alias = "Json", alias = "JSON")]
    Json,
    #[serde(alias = "Xml", alias = "XML")]
    Xml,
    #[serde(alias = "Toml", alias = "TOML")]
    Toml,
}

impl std::fmt::Display for CodeLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeLanguage::Typescript => write!(f, "typescript"),
            CodeLanguage::Python => write!(f, "python"),
            CodeLanguage::Javascript => write!(f, "javascript"),
            CodeLanguage::Java => write!(f, "java"),
            CodeLanguage::Rust => write!(f, "rust"),
            CodeLanguage::Go => write!(f, "go"),
            CodeLanguage::Cpp => write!(f, "cpp"),
            CodeLanguage::C => write!(f, "c"),
            CodeLanguage::Csharp => write!(f, "csharp"),
            CodeLanguage::Ruby => write!(f, "ruby"),
            CodeLanguage::Php => write!(f, "php"),
            CodeLanguage::Swift => write!(f, "swift"),
            CodeLanguage::Kotlin => write!(f, "kotlin"),
            CodeLanguage::Scala => write!(f, "scala"),
            CodeLanguage::Shell => write!(f, "shell"),
            CodeLanguage::Html => write!(f, "html"),
            CodeLanguage::Css => write!(f, "css"),
            CodeLanguage::Jsx => write!(f, "jsx"),
            CodeLanguage::Tsx => write!(f, "tsx"),
            CodeLanguage::Haskell => write!(f, "haskell"),
            CodeLanguage::Elixir => write!(f, "elixir"),
            CodeLanguage::Erlang => write!(f, "erlang"),
            CodeLanguage::Clojure => write!(f, "clojure"),
            CodeLanguage::Fsharp => write!(f, "fsharp"),
            CodeLanguage::Ocaml => write!(f, "ocaml"),
            CodeLanguage::Lisp => write!(f, "lisp"),
            CodeLanguage::Scheme => write!(f, "scheme"),
            CodeLanguage::R => write!(f, "r"),
            CodeLanguage::Julia => write!(f, "julia"),
            CodeLanguage::Matlab => write!(f, "matlab"),
            CodeLanguage::Octave => write!(f, "octave"),
            CodeLanguage::Sql => write!(f, "sql"),
            CodeLanguage::Plsql => write!(f, "plsql"),
            CodeLanguage::Zig => write!(f, "zig"),
            CodeLanguage::Nim => write!(f, "nim"),
            CodeLanguage::Crystal => write!(f, "crystal"),
            CodeLanguage::D => write!(f, "d"),
            CodeLanguage::Perl => write!(f, "perl"),
            CodeLanguage::Lua => write!(f, "lua"),
            CodeLanguage::Groovy => write!(f, "groovy"),
            CodeLanguage::Powershell => write!(f, "powershell"),
            CodeLanguage::Dart => write!(f, "dart"),
            CodeLanguage::Objectivec => write!(f, "objectivec"),
            CodeLanguage::Fortran => write!(f, "fortran"),
            CodeLanguage::Cobol => write!(f, "cobol"),
            CodeLanguage::Pascal => write!(f, "pascal"),
            CodeLanguage::Ada => write!(f, "ada"),
            CodeLanguage::Vb => write!(f, "vb"),
            CodeLanguage::Assembly => write!(f, "assembly"),
            CodeLanguage::Prolog => write!(f, "prolog"),
            CodeLanguage::Yaml => write!(f, "yaml"),
            CodeLanguage::Json => write!(f, "json"),
            CodeLanguage::Xml => write!(f, "xml"),
            CodeLanguage::Toml => write!(f, "toml"),
        }
    }
}

impl CodeLanguage {
    pub fn to_dynamic_tool_type(&self) -> Option<DynamicToolType> {
        match self {
            // Deno runtime for JavaScript/TypeScript
            CodeLanguage::Typescript | CodeLanguage::Javascript |
            CodeLanguage::Jsx | CodeLanguage::Tsx => Some(DynamicToolType::DenoDynamic),

            // Python runtime
            CodeLanguage::Python => Some(DynamicToolType::PythonDynamic),

            // Docker runtime for compiled/other languages
            CodeLanguage::Java | CodeLanguage::Rust | CodeLanguage::Go |
            CodeLanguage::Cpp | CodeLanguage::C | CodeLanguage::Csharp |
            CodeLanguage::Ruby | CodeLanguage::Php | CodeLanguage::Swift |
            CodeLanguage::Kotlin | CodeLanguage::Scala | CodeLanguage::Shell |
            CodeLanguage::Haskell | CodeLanguage::Elixir | CodeLanguage::Erlang |
            CodeLanguage::Clojure | CodeLanguage::Fsharp | CodeLanguage::Ocaml |
            CodeLanguage::Lisp | CodeLanguage::Scheme | CodeLanguage::R |
            CodeLanguage::Julia | CodeLanguage::Matlab | CodeLanguage::Octave |
            CodeLanguage::Sql | CodeLanguage::Plsql | CodeLanguage::Zig |
            CodeLanguage::Nim | CodeLanguage::Crystal | CodeLanguage::D |
            CodeLanguage::Perl | CodeLanguage::Lua | CodeLanguage::Groovy |
            CodeLanguage::Powershell | CodeLanguage::Dart | CodeLanguage::Objectivec |
            CodeLanguage::Fortran | CodeLanguage::Cobol | CodeLanguage::Pascal |
            CodeLanguage::Ada | CodeLanguage::Vb | CodeLanguage::Assembly |
            CodeLanguage::Prolog => Some(DynamicToolType::DockerDynamic),

            // Static/markup languages don't need runtime
            CodeLanguage::Html | CodeLanguage::Css | CodeLanguage::Yaml |
            CodeLanguage::Json | CodeLanguage::Xml | CodeLanguage::Toml => None,
        }
    }

    pub fn get_file_extension(&self) -> &str {
        match self {
            CodeLanguage::Typescript => "ts",
            CodeLanguage::Python => "py",
            CodeLanguage::Javascript => "js",
            CodeLanguage::Java => "java",
            CodeLanguage::Rust => "rs",
            CodeLanguage::Go => "go",
            CodeLanguage::Cpp => "cpp",
            CodeLanguage::C => "c",
            CodeLanguage::Csharp => "cs",
            CodeLanguage::Ruby => "rb",
            CodeLanguage::Php => "php",
            CodeLanguage::Swift => "swift",
            CodeLanguage::Kotlin => "kt",
            CodeLanguage::Scala => "scala",
            CodeLanguage::Shell => "sh",
            CodeLanguage::Html => "html",
            CodeLanguage::Css => "css",
            CodeLanguage::Jsx => "jsx",
            CodeLanguage::Tsx => "tsx",
            CodeLanguage::Haskell => "hs",
            CodeLanguage::Elixir => "ex",
            CodeLanguage::Erlang => "erl",
            CodeLanguage::Clojure => "clj",
            CodeLanguage::Fsharp => "fs",
            CodeLanguage::Ocaml => "ml",
            CodeLanguage::Lisp => "lisp",
            CodeLanguage::Scheme => "scm",
            CodeLanguage::R => "r",
            CodeLanguage::Julia => "jl",
            CodeLanguage::Matlab => "m",
            CodeLanguage::Octave => "m",
            CodeLanguage::Sql => "sql",
            CodeLanguage::Plsql => "pls",
            CodeLanguage::Zig => "zig",
            CodeLanguage::Nim => "nim",
            CodeLanguage::Crystal => "cr",
            CodeLanguage::D => "d",
            CodeLanguage::Perl => "pl",
            CodeLanguage::Lua => "lua",
            CodeLanguage::Groovy => "groovy",
            CodeLanguage::Powershell => "ps1",
            CodeLanguage::Dart => "dart",
            CodeLanguage::Objectivec => "m",
            CodeLanguage::Fortran => "f90",
            CodeLanguage::Cobol => "cob",
            CodeLanguage::Pascal => "pas",
            CodeLanguage::Ada => "adb",
            CodeLanguage::Vb => "vb",
            CodeLanguage::Assembly => "asm",
            CodeLanguage::Prolog => "pl",
            CodeLanguage::Yaml => "yaml",
            CodeLanguage::Json => "json",
            CodeLanguage::Xml => "xml",
            CodeLanguage::Toml => "toml",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_language_serialization() {
        // Test positive cases
        let typescript = CodeLanguage::Typescript;
        let python = CodeLanguage::Python;

        // Serialize
        let typescript_str = serde_json::to_string(&typescript).unwrap();
        let python_str = serde_json::to_string(&python).unwrap();

        assert_eq!(typescript_str, "\"typescript\"");
        assert_eq!(python_str, "\"python\"");

        // Deserialize
        let typescript_deserialized: CodeLanguage = serde_json::from_str(&typescript_str).unwrap();
        let python_deserialized: CodeLanguage = serde_json::from_str(&python_str).unwrap();

        assert_eq!(typescript_deserialized, CodeLanguage::Typescript);
        assert_eq!(python_deserialized, CodeLanguage::Python);

        // Test case variations
        let case_variations = vec![
            ("\"typescript\"", CodeLanguage::Typescript),
            ("\"Typescript\"", CodeLanguage::Typescript),
            ("\"TYPESCRIPT\"", CodeLanguage::Typescript),
            ("\"python\"", CodeLanguage::Python),
            ("\"Python\"", CodeLanguage::Python),
            ("\"PYTHON\"", CodeLanguage::Python),
        ];

        for (input, expected) in case_variations {
            let result: CodeLanguage = serde_json::from_str(input).unwrap();
            assert_eq!(result, expected, "Failed to deserialize: {}", input);
        }

        // Test negative cases
        let invalid_cases = vec!["\"invalid\"", "TypeScript", "123", "null", "\"\""];

        for invalid_case in invalid_cases {
            let result = serde_json::from_str::<CodeLanguage>(invalid_case);
            assert!(result.is_err(), "Should fail to deserialize: {}", invalid_case);
        }
    }
}
