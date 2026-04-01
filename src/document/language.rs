use strum_macros::{Display, EnumString};

/// Language type wrapper from lsp
/// Reference to https://code.visualstudio.com/docs/languages/identifiers
#[allow(non_camel_case_types)]
#[derive(EnumString, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Language {
    #[strum(serialize = "ascii-doc")]
    ascii_doc,
    astro,
    bash,
    biome,
    c,
    csharp,
    #[strum(serialize = "cpp")]
    c_plus_plus,
    clojure,
    css,
    csv,
    crystal,
    d,
    dart,
    deno,
    dockerfile,
    elixir,
    elm,
    emmet,
    erlang,
    fish,
    fsharp,
    gdscript,
    #[strum(serialize = "git-commit")]
    git_commit,
    gleam,
    glsl,
    go,
    graphql,
    groovy,
    haskell,
    heex,
    html,
    hy,
    idris,
    java,
    javascript,
    json,
    jsonc,
    julia,
    kotlin,
    latex,
    lua,
    luau,
    makefile,
    markdown,
    nim,
    nix,
    ocaml,
    php,
    #[strum(serialize = "plain-text")]
    plain_text,
    prisma,
    proto,
    pure_script,
    python,
    r,
    racket,
    rego,
    rest,
    re_structured_text,
    roc,
    ruby,
    rust,
    scala,
    scheme,
    scss,
    #[strum(serialize = "shell-script")]
    shell_script,
    sql,
    svelte,
    swift,
    tailwindcss,
    terraform,
    toml,
    tsx,
    typescript,
    typst,
    uiua,
    vue,
    wit,
    xml,
    yaml,
    yarn,
    zig,
    // For parse failed
    #[strum(default)]
    Unknown(String),
}

impl Language {
    pub fn from_id(language_id: &str) -> Language {
        Language::from(language_id)
    }
}
