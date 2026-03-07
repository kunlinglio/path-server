use strum_macros::{Display, EnumString};

/// Language type wrapper from lsp
#[allow(non_camel_case_types)]
#[derive(EnumString, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub enum Language {
    AsciiDoc,
    Astro,
    Bash,
    Biome,
    C,
    #[strum(serialize = "C#")]
    CSharp,
    #[strum(serialize = "C++")]
    CPlusPlus,
    Clojure,
    //CSharp,
    CSS,
    CSV,
    Crystal,
    D,
    Dart,
    Deno,
    Docker,
    Elixir,
    Elm,
    Emmet,
    Erlang,
    Fish,
    FSharp,
    GDScript,
    #[strum(serialize = "Git Commit")]
    GitCommit,
    Gleam,
    GLSL,
    Go,
    GraphQL,
    Groovy,
    Haskell,
    HEEX,
    HTML,
    Hy,
    Idris,
    Java,
    JavaScript,
    JSON,
    JSONC,
    Julia,
    Kotlin,
    LaTeX,
    Lua,
    Luau,
    Makefile,
    Markdown,
    Nim,
    Nix,
    OCaml,
    PHP,
    #[strum(serialize = "Plain Text")]
    PlainText,
    Prisma,
    Proto,
    PureScript,
    Python,
    R,
    Racket,
    Rego,
    reST,
    ReStructuredText,
    Roc,
    Ruby,
    Rust,
    Scala,
    Scheme,
    SCSS,
    #[strum(serialize = "Shell Script")]
    ShellScript,
    SQL,
    Svelte,
    Swift,
    TailwindCSS,
    Terraform,
    TOML,
    TSX,
    TypeScript,
    Typst,
    Uiua,
    Vue,
    #[strum(serialize = "Vue.js")]
    VueJs,
    WIT,
    XML,
    YAML,
    Yarn,
    Zig,
    // For parse failed
    Unknown,
}

impl Language {
    pub fn from_id(language_id: &str) -> Language {
        Language::try_from(language_id).unwrap_or(Language::Unknown)
    }
}
