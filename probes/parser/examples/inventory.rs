use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use vhdl_syntax::{
    parser,
    standard::VHDLStandard,
    syntax::{AstNode, NodeKind, SyntaxNode},
    tokens::TokenKind,
};

#[derive(Debug, PartialEq, Eq)]
struct Declaration {
    kind: NodeKind,
    line: usize,
    name: String,
    signature: String,
}

fn declarations(source: &[u8]) -> Result<Vec<Declaration>, String> {
    let (design, errors) = parser::parse_with_standard(VHDLStandard::VHDL2008, source);
    if !errors.is_empty() {
        return Err(parser::error::display_errors(&errors));
    }
    let mut result = Vec::new();
    collect(&design.raw(), source, &mut result)?;
    Ok(result)
}

fn collect(node: &SyntaxNode, source: &[u8], result: &mut Vec<Declaration>) -> Result<(), String> {
    match node.kind() {
        NodeKind::PackageBody | NodeKind::SubprogramBody => {
            return Err("package bodies are not declaration inventory inputs".to_owned());
        }
        NodeKind::SubprogramDeclaration
        | NodeKind::AliasDeclaration
        | NodeKind::FullTypeDeclaration
        | NodeKind::IncompleteTypeDeclaration
        | NodeKind::SubtypeDeclaration
        | NodeKind::ConstantDeclaration
        | NodeKind::FileDeclaration
        | NodeKind::AttributeDeclaration
        | NodeKind::AttributeSpecification
        | NodeKind::ContextDeclaration
        | NodeKind::GenericClause
        | NodeKind::PackageInstantiationDeclaration
        | NodeKind::UseClause => {
            let range = node.text_range();
            let tokens: Vec<_> = node
                .descendants_with_tokens()
                .filter_map(|element| element.as_token())
                .map(|token| {
                    let text: String = source[token.text_range()]
                        .iter()
                        .copied()
                        .map(char::from)
                        .collect();
                    match token.kind() {
                        TokenKind::Identifier if !text.starts_with('\\') => text.to_lowercase(),
                        TokenKind::Keyword(_) => text.to_ascii_lowercase(),
                        _ => text,
                    }
                })
                .collect();
            let name_position = tokens
                .iter()
                .position(|token| token == "function" || token == "procedure")
                .map_or(1, |position| position + 1);
            result.push(Declaration {
                kind: node.kind(),
                line: source[..range.start]
                    .iter()
                    .filter(|&&byte| byte == b'\n')
                    .count()
                    + 1,
                name: tokens.get(name_position).cloned().unwrap_or_default(),
                signature: tokens.join(" "),
            });
            return Ok(());
        }
        _ => {}
    }
    for child in node.children() {
        collect(&child, source, result)?;
    }
    Ok(())
}

const RUST_HDL_REVISION: &str = "dc8c090f7aeb9a95149eadc5e8b48f170382a8d4";
const GHDL_REVISION: &str = "a977191e75d345ee30a936279f425265e52b37b3";
const NVC_REVISION: &str = "8630448e4a25aa4bd74066b94eba08063597a3ca";

struct Package {
    name: &'static str,
    rust: &'static str,
    ghdl: Option<&'static str>,
    nvc: Option<&'static str>,
}

const PACKAGES: &[Package] = &[
    Package {
        name: "std.standard",
        rust: "std/standard.vhd",
        ghdl: None,
        nvc: Some("std.08/standard.vhd"),
    },
    Package {
        name: "std.textio",
        rust: "std/textio.vhd",
        ghdl: Some("std/textio.vhdl"),
        nvc: Some("std.08/textio.vhd"),
    },
    Package {
        name: "std.env",
        rust: "std/env.vhd",
        ghdl: Some("std/env.vhdl"),
        nvc: Some("std.08/env.vhd"),
    },
    Package {
        name: "ieee.std_logic_1164",
        rust: "ieee2008/std_logic_1164.vhdl",
        ghdl: Some("ieee2008/std_logic_1164.vhdl"),
        nvc: Some("ieee.08/std_logic_1164.vhdl"),
    },
    Package {
        name: "ieee.numeric_std",
        rust: "ieee2008/numeric_std.vhdl",
        ghdl: Some("ieee2008/numeric_std.vhdl"),
        nvc: Some("ieee.08/numeric_std.vhdl"),
    },
    Package {
        name: "ieee.math_real",
        rust: "ieee2008/math_real.vhdl",
        ghdl: Some("ieee2008/math_real.vhdl"),
        nvc: Some("ieee/math_real.vhdl"),
    },
    Package {
        name: "ieee.fixed_float_types",
        rust: "ieee2008/fixed_float_types.vhdl",
        ghdl: Some("ieee2008/fixed_float_types.vhdl"),
        nvc: Some("ieee.08/fixed_float_types.vhdl"),
    },
    Package {
        name: "ieee.fixed_generic_pkg",
        rust: "ieee2008/fixed_generic_pkg.vhdl",
        ghdl: Some("ieee2008/fixed_generic_pkg.vhdl"),
        nvc: Some("ieee.08/fixed_generic_pkg.vhdl"),
    },
    Package {
        name: "ieee.fixed_pkg",
        rust: "ieee2008/fixed_pkg.vhdl",
        ghdl: Some("ieee2008/fixed_pkg.vhdl"),
        nvc: Some("ieee.08/fixed_pkg.vhdl"),
    },
    Package {
        name: "ieee.float_generic_pkg",
        rust: "ieee2008/float_generic_pkg.vhdl",
        ghdl: Some("ieee2008/float_generic_pkg.vhdl"),
        nvc: Some("ieee.08/float_generic_pkg.vhdl"),
    },
    Package {
        name: "ieee.float_pkg",
        rust: "ieee2008/float_pkg.vhdl",
        ghdl: Some("ieee2008/float_pkg.vhdl"),
        nvc: Some("ieee.08/float_pkg.vhdl"),
    },
    Package {
        name: "ieee.math_complex",
        rust: "ieee2008/math_complex.vhdl",
        ghdl: Some("ieee2008/math_complex.vhdl"),
        nvc: Some("ieee/math_complex.vhdl"),
    },
    Package {
        name: "ieee.numeric_bit",
        rust: "ieee2008/numeric_bit.vhdl",
        ghdl: Some("ieee2008/numeric_bit.vhdl"),
        nvc: Some("ieee.08/numeric_bit.vhdl"),
    },
    Package {
        name: "ieee.numeric_bit_unsigned",
        rust: "ieee2008/numeric_bit_unsigned.vhdl",
        ghdl: Some("ieee2008/numeric_bit_unsigned.vhdl"),
        nvc: Some("ieee.08/numeric_bit_unsigned.vhdl"),
    },
    Package {
        name: "ieee.numeric_std_unsigned",
        rust: "ieee2008/numeric_std_unsigned.vhdl",
        ghdl: Some("ieee2008/numeric_std_unsigned.vhdl"),
        nvc: Some("ieee.08/numeric_std_unsigned.vhdl"),
    },
    Package {
        name: "ieee.std_logic_textio",
        rust: "ieee2008/std_logic_textio.vhdl",
        ghdl: Some("ieee2008/std_logic_textio.vhdl"),
        nvc: Some("ieee.08/std_logic_textio.vhdl"),
    },
    Package {
        name: "ieee.ieee_bit_context",
        rust: "ieee2008/ieee_bit_context.vhdl",
        ghdl: Some("ieee2008/ieee_bit_context.vhdl"),
        nvc: Some("ieee.08/ieee_bit_context.vhdl"),
    },
    Package {
        name: "ieee.ieee_std_context",
        rust: "ieee2008/ieee_std_context.vhdl",
        ghdl: Some("ieee2008/ieee_std_context.vhdl"),
        nvc: Some("ieee.08/ieee_std_context.vhdl"),
    },
];

fn select_2008(source: &[u8]) -> Result<Vec<u8>, String> {
    let mut result = Vec::with_capacity(source.len());
    let mut blocks = Vec::new();
    for line in source.split_inclusive(|&byte| byte == b'\n') {
        let text: String = line.iter().copied().map(char::from).collect();
        let trimmed = text.trim();
        match trimmed {
            "--START-V08" => blocks.push("08"),
            "--START-V19" => blocks.push("19"),
            "--END-V08" | "--END-V19" => {
                if blocks.pop() != Some(&trimmed[7..]) {
                    return Err(format!("unbalanced reference marker: {trimmed}"));
                }
            }
            marker if marker.starts_with("--START-") || marker.starts_with("--END-") => {
                return Err(format!("unrecognized reference marker: {marker}"));
            }
            _ => {}
        }
        let include = !blocks.contains(&"19") && !trimmed.ends_with("--V87");
        result.extend(line.iter().map(|&byte| {
            if include || byte == b'\n' || byte == b'\r' {
                byte
            } else {
                b' '
            }
        }));
    }
    if !blocks.is_empty() {
        return Err("unclosed reference version block".to_owned());
    }
    Ok(result)
}

fn is_text_io(name: &str) -> bool {
    matches!(
        name,
        "read"
            | "write"
            | "readline"
            | "writeline"
            | "tee"
            | "sread"
            | "bread"
            | "bwrite"
            | "oread"
            | "owrite"
            | "hread"
            | "hwrite"
            | "string_read"
            | "string_write"
            | "swrite"
            | "binary_read"
            | "binary_write"
            | "octal_read"
            | "octal_write"
            | "hex_read"
            | "hex_write"
    )
}

fn owner(package: &str, declaration: &Declaration) -> Result<(u16, &'static str), String> {
    let name = declaration.name.trim_matches('"').to_ascii_lowercase();
    if package == "std.textio" && matches!(name.as_str(), "file_rewind" | "file_mode" | "file_size")
    {
        return Ok((15, "excluded-2019"));
    }
    if name == "never_waits" || declaration.kind == NodeKind::AttributeSpecification {
        return Ok((110, "reference-annotation"));
    }
    if declaration.kind == NodeKind::ContextDeclaration {
        return Ok((103, "ieee-context"));
    }
    if declaration.kind == NodeKind::UseClause {
        return Ok((39, "visibility"));
    }
    if declaration.kind == NodeKind::GenericClause {
        return Ok((72, "generic-policy"));
    }
    if declaration.kind == NodeKind::PackageInstantiationDeclaration {
        return Ok((47, "package-instance"));
    }
    if is_text_io(&name) {
        let core = package == "std.textio"
            && matches!(
                name.as_str(),
                "read" | "write" | "readline" | "writeline" | "tee"
            );
        return Ok((if core { 107 } else { 108 }, "text-io"));
    }
    if package == "std.env" {
        return Ok((59, "environment"));
    }
    let base = match package {
        "std.standard" => 17,
        "std.textio" => 107,
        "ieee.std_logic_1164" => 61,
        "ieee.numeric_std" => 63,
        "ieee.math_real" => 69,
        "ieee.fixed_float_types" => 72,
        "ieee.fixed_generic_pkg" => 73,
        "ieee.float_generic_pkg" => 77,
        "ieee.math_complex" => 81,
        "ieee.numeric_bit" => 88,
        "ieee.numeric_bit_unsigned" => 94,
        "ieee.numeric_std_unsigned" => 98,
        _ => return Err(format!("unassigned package: {package}")),
    };
    if package == "std.standard" && matches!(name.as_str(), "integer" | "real" | "time") {
        return Ok((3, "implementation-limit"));
    }
    if declaration.kind != NodeKind::SubprogramDeclaration {
        return Ok((base, "declaration-or-alias"));
    }
    let assigned = match package {
        "std.standard" => match name.as_str() {
            "now" => (25, "time"),
            "rising_edge" | "falling_edge" => (57, "signal-observation"),
            _ => return Err(format!("unassigned standard function: {name}")),
        },
        "std.textio" if name == "justify" => (107, "formatting"),
        "ieee.std_logic_1164" => match name.as_str() {
            "resolved" => (52, "resolution"),
            "rising_edge" | "falling_edge" | "is_x" | "??" | "sll" | "srl" | "rol" | "ror" => {
                (62, "logic-helpers")
            }
            "and" | "or" | "nand" | "nor" | "xor" | "xnor" | "not" | "to_bit" | "to_bitvector"
            | "to_stdulogic" | "to_stdlogicvector" | "to_stdulogicvector" | "to_01" | "to_x01"
            | "to_x01z" | "to_ux01" | "to_ostring" | "to_hstring" => (61, "logic-conversion"),
            _ => return Err(format!("unassigned logic function: {name}")),
        },
        "ieee.numeric_std" => match name.as_str() {
            "+" | "-" | "abs" => (64, "numeric-additive"),
            "*" | "/" | "mod" | "rem" => (65, "numeric-multiplicative"),
            "shift_left" | "shift_right" | "rotate_left" | "rotate_right" | "sll" | "srl"
            | "sla" | "sra" | "rol" | "ror" => (67, "numeric-shift"),
            "=" | "/=" | "<" | "<=" | ">" | ">=" | "?=" | "?/=" | "?<" | "?<=" | "?>" | "?>="
            | "minimum" | "maximum" | "std_match" | "find_leftmost" | "find_rightmost" | "is_x"
            | "and" | "or" | "nand" | "nor" | "xor" | "xnor" | "not" => {
                (66, "numeric-comparison-logic")
            }
            "resize" | "to_integer" | "to_unsigned" | "to_signed" | "to_01" | "to_x01"
            | "to_x01z" | "to_ux01" | "to_ostring" | "to_hstring" => (63, "numeric-conversion"),
            _ => return Err(format!("unassigned numeric function: {name}")),
        },
        "ieee.numeric_bit" | "ieee.numeric_bit_unsigned" | "ieee.numeric_std_unsigned" => {
            if matches!(name.as_str(), "rising_edge" | "falling_edge") {
                (91, "bit-signal-observation")
            } else if matches!(
                name.as_str(),
                "to_bitvector"
                    | "to_bit_vector"
                    | "to_bv"
                    | "to_slv"
                    | "to_sulv"
                    | "to_stdlogicvector"
                    | "to_stdulogicvector"
                    | "to_std_logic_vector"
                    | "to_std_ulogic_vector"
            ) {
                (base, "unsigned-conversion")
            } else {
                let (numeric_owner, _) = owner("ieee.numeric_std", declaration)?;
                let assigned = match (package, numeric_owner) {
                    ("ieee.numeric_bit", 63) => 88,
                    ("ieee.numeric_bit", 64) => 89,
                    ("ieee.numeric_bit", 65) => 90,
                    ("ieee.numeric_bit", 66) => 91,
                    ("ieee.numeric_bit", 67) => 92,
                    ("ieee.numeric_bit_unsigned", 63) => 94,
                    ("ieee.numeric_bit_unsigned", 64 | 65) => 95,
                    ("ieee.numeric_bit_unsigned", 66 | 67) => 96,
                    ("ieee.numeric_std_unsigned", 63) => 98,
                    ("ieee.numeric_std_unsigned", 64 | 65) => 99,
                    ("ieee.numeric_std_unsigned", 66) => 100,
                    ("ieee.numeric_std_unsigned", 67) => 101,
                    _ => return Err(format!("unassigned numeric wrapper: {package}::{name}")),
                };
                (assigned, "numeric-bit-or-unsigned")
            }
        }
        "ieee.math_complex" => match name.as_str() {
            "cmplx"
            | "complex_to_polar"
            | "polar_to_complex"
            | "get_principal_value"
            | "abs"
            | "arg"
            | "conj"
            | "="
            | "/=" => (82, "complex-conversion-helper"),
            "+" | "-" => (83, "complex-additive"),
            "*" | "/" => (84, "complex-multiplicative"),
            "exp" | "log" | "log2" | "log10" | "sqrt" => (85, "complex-elementary"),
            "sin" | "cos" | "sinh" | "cosh" => (86, "complex-trigonometric"),
            _ => return Err(format!("unassigned complex function: {name}")),
        },
        "ieee.math_real" => match name.as_str() {
            "uniform" => (71, "random-state"),
            "sin" | "cos" | "tan" | "arcsin" | "arccos" | "arctan" | "sinh" | "cosh" | "tanh"
            | "arcsinh" | "arccosh" | "arctanh" => (70, "real-trigonometric"),
            "sign" | "ceil" | "floor" | "round" | "trunc" | "realmax" | "realmin" | "mod"
            | "sqrt" | "cbrt" | "**" | "exp" | "log" | "log2" | "log10" => (69, "real-elementary"),
            _ => return Err(format!("unassigned real function: {name}")),
        },
        "ieee.fixed_generic_pkg" => match name.as_str() {
            "+" | "-" | "abs" | "*" | "add_carry" => (74, "fixed-arithmetic"),
            "/" | "mod" | "rem" | "divide" | "reciprocal" | "remainder" | "modulo" => {
                (75, "fixed-division")
            }
            "resize" | "saturate" | "to_ufixed" | "to_sfixed" | "to_ufix" | "to_sfix"
            | "to_real" | "to_integer" | "to_unsigned" | "to_signed" | "to_slv" | "to_sulv"
            | "to_01" | "to_x01" | "to_x01z" | "to_ux01" => (73, "fixed-conversion"),
            "=" | "/=" | "<" | "<=" | ">" | ">=" | "?=" | "?/=" | "?<" | "?<=" | "?>" | "?>="
            | "minimum" | "maximum" | "std_match" | "find_leftmost" | "find_rightmost" | "is_x"
            | "is_negative" | "and" | "or" | "nand" | "nor" | "xor" | "xnor" | "not"
            | "shift_left" | "shift_right" | "sll" | "srl" | "sla" | "sra" | "rol" | "ror"
            | "scalb" | "ufixed_high" | "ufixed_low" | "sfixed_high" | "sfixed_low"
            | "ufix_high" | "ufix_low" | "sfix_high" | "sfix_low" | "to_string" | "to_ostring"
            | "to_hstring" | "from_string" | "from_ostring" | "from_hstring" => {
                (76, "fixed-helpers")
            }
            _ => return Err(format!("unassigned fixed function: {name}")),
        },
        "ieee.float_generic_pkg" => {
            match name.as_str() {
                "+" | "-" | "abs" | "add" | "subtract" => (78, "float-additive"),
                "*" | "/" | "multiply" | "divide" | "sqrt" | "reciprocal" | "mac" => {
                    (79, "float-multiplicative")
                }
                "resize" | "to_float" | "to_float32" | "to_float64" | "to_float128" | "to_real"
                | "to_integer" | "to_unsigned" | "to_signed" | "to_ufixed" | "to_sfixed"
                | "to_slv" | "to_sulv" | "to_01" | "to_x01" | "to_x01z" | "to_ux01" | "classfp"
                | "isnan" | "finite" | "is_negative" | "is_x" | "unordered" | "bitstoreal"
                | "realtobits" => (77, "float-format-conversion"),
                "=" | "/=" | "<" | "<=" | ">" | ">=" | "?=" | "?/=" | "?<" | "?<=" | "?>"
                | "?>=" | "minimum" | "maximum" | "std_match" | "find_leftmost"
                | "find_rightmost" | "and" | "or" | "nand" | "nor" | "xor" | "xnor" | "not"
                | "eq" | "ne" | "lt" | "gt" | "le" | "ge" | "mod" | "rem" | "modulo"
                | "remainder" | "normalize" | "break_number" | "logb" | "scalb" | "dividebyp2"
                | "copysign" | "nextafter" | "nanfp" | "qnanfp" | "neg_inffp" | "pos_inffp"
                | "zerofp" | "neg_zerofp" | "to_string" | "to_ostring" | "to_hstring"
                | "from_string" | "from_ostring" | "from_hstring" => (80, "float-helpers"),
                _ => return Err(format!("unassigned float function: {name}")),
            }
        }
        _ => return Err(format!("unassigned function in {package}: {name}")),
    };
    Ok(assigned)
}

struct Record {
    declaration: Declaration,
    sources: Vec<String>,
    origins: String,
}

fn add_source(
    records: &mut BTreeMap<String, Record>,
    root: &Path,
    relative: &str,
    repository: &str,
    revision: &str,
    origin: char,
    template: bool,
) -> Result<usize, Box<dyn Error>> {
    let source = fs::read(root.join(relative))?;
    let pinned = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("show")
        .arg(format!("{revision}:{relative}"))
        .output()?;
    if !pinned.status.success() || pinned.stdout != source {
        return Err(format!("declaration file differs from pinned Git blob: {relative}").into());
    }
    let selected = if template {
        select_2008(&source)?
    } else {
        source
    };
    let declarations = declarations(&selected).map_err(|error| format!("{relative}: {error}"))?;
    let count = declarations.len();
    for declaration in declarations {
        let key = format!("{:?}:{}", declaration.kind, declaration.signature);
        let location = format!(
            "[{origin}:{}](https://github.com/{repository}/blob/{revision}/{relative}#L{})",
            declaration.line, declaration.line
        );
        let record = records.entry(key).or_insert_with(|| Record {
            declaration,
            sources: Vec::new(),
            origins: String::new(),
        });
        if record.origins.contains(origin) {
            return Err(format!("duplicate declaration in {relative}").into());
        }
        record.sources.push(location);
        record.origins.push(origin);
    }
    Ok(count)
}

fn inline_code(text: &str) -> String {
    let mut escaped = String::new();
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '|' => escaped.push_str("&#124;"),
            '`' => escaped.push_str("&#96;"),
            character if !character.is_ascii() => {
                write!(escaped, "&#{};", u32::from(character)).unwrap();
            }
            character => escaped.push(character),
        }
    }
    format!("<code>{escaped}</code>")
}

fn inventory_report(rust: &Path, ghdl: &Path, nvc: &Path) -> Result<String, Box<dyn Error>> {
    let mut output = String::from(
        "<!-- markdownlint-configure-file { \"MD033\": { \"allowed_elements\": [\"code\"] } } -->\n\n\
         # Native Declaration Inventory\n\n\
         Generated by the parser probe's inventory example. Do not edit rows manually.\n\n\
         R = Rust-HDL master, G = GHDL 2008-selected declarations, N = NVC std.08 / ieee.08 / inherited ieee math.\n\
         See [coverage.md](coverage.md) for revisions, scope decisions, dependencies, implicit operations, and reproduction commands.\n\n\
         Every test ID below is reserved, not implemented. All required native behavior is planned.\n\
         The report is an audit of source profiles, not an instruction to merge duplicate source variants into one package.\n\
         A/E/R means eligible for analysis/elaboration/runtime evaluation only when staticness and purity permit it.\n\
         Reference annotations and excluded-2019 entries are not native API requirements. Implementation limits remain decisions for chunk 003.\n\n",
    );
    for package in PACKAGES {
        let mut records = BTreeMap::new();
        let rust_count = add_source(
            &mut records,
            rust,
            &format!("vhdl_libraries/{}", package.rust),
            "VHDL-LS/rust_hdl",
            RUST_HDL_REVISION,
            'R',
            false,
        )?;
        let ghdl_count = if let Some(path) = package.ghdl {
            add_source(
                &mut records,
                ghdl,
                &format!("libraries/{path}"),
                "ghdl/ghdl",
                GHDL_REVISION,
                'G',
                package.name.starts_with("std."),
            )?
        } else {
            0
        };
        let nvc_count = if let Some(path) = package.nvc {
            add_source(
                &mut records,
                nvc,
                &format!("lib/{path}"),
                "nickg/nvc",
                NVC_REVISION,
                'N',
                false,
            )?
        } else {
            0
        };
        writeln!(output, "## {}\n", package.name)?;
        writeln!(output, "Parsed declarations: R={rust_count}, G={ghdl_count}, N={nvc_count}. Audited profiles: {}.\n", records.len())?;
        output.push_str("| ID / Reserved Test | Kind | Full Profile | Family / Chunk | Evaluation | Scope / Origins | Sources |\n| --- | --- | --- | --- | --- | --- | --- |\n");
        if records.is_empty() {
            if package.name != "ieee.std_logic_textio" {
                return Err(format!("unexpected empty native unit: {}", package.name).into());
            }
            writeln!(output, "| `native.ieee.std_logic_textio.unit` | PackageUnit | Empty VHDL-2008 compatibility package; I/O is declared by std_logic_1164 | compatibility / `103` | A/E | planned / RGN | [R](https://github.com/VHDL-LS/rust_hdl/blob/{RUST_HDL_REVISION}/vhdl_libraries/{}), [G](https://github.com/ghdl/ghdl/blob/{GHDL_REVISION}/libraries/{}), [N](https://github.com/nickg/nvc/blob/{NVC_REVISION}/lib/{}) |", package.rust, package.ghdl.unwrap(), package.nvc.unwrap())?;
        }
        for (index, record) in records.values().enumerate() {
            let declaration = &record.declaration;
            let (chunk, family) = owner(package.name, declaration)?;
            let provenance = declaration.kind == NodeKind::ConstantDeclaration
                && declaration.name == "copyrightnotice";
            if package.name.starts_with("ieee.")
                && record.origins.contains('R') != record.origins.contains('G')
                && !provenance
            {
                return Err(format!(
                    "unreconciled IEEE profile in {}: {} ({})",
                    package.name, declaration.signature, record.origins
                )
                .into());
            }
            let runtime = is_text_io(&declaration.name)
                || package.name == "std.env"
                || declaration.name == "uniform"
                || declaration.signature.starts_with("impure ")
                || declaration.signature.starts_with("procedure ")
                || declaration.signature.contains("signal ");
            let evaluation = if runtime {
                "R"
            } else if declaration.kind == NodeKind::SubprogramDeclaration {
                "A/E/R"
            } else {
                "A/E"
            };
            let status = match family {
                "excluded-2019" => "excluded-2019",
                "reference-annotation" => "annotation-only",
                "implementation-limit" => "implementation-defined",
                _ => "planned",
            };
            let test = format!("native.{}.{:04}", package.name, index + 1);
            writeln!(output, "| `{test}` | {:?} | {} | {family} / `{chunk:03}` | {evaluation} | {status} / {} | {} |", declaration.kind, inline_code(&declaration.signature), record.origins, record.sources.join(", "))?;
        }
        output.push('\n');
    }
    Ok(output)
}

fn verify_repository(root: &Path, revision: &str) -> Result<(), Box<dyn Error>> {
    let head = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !head.status.success() || String::from_utf8(head.stdout)?.trim() != revision {
        return Err(format!("reference revision mismatch: expected {revision}").into());
    }
    Ok(())
}

fn check_report(expected: &str, actual: &str) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err("inventory differs: review declarations, ownership, or source revisions before regenerating".to_owned())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    if arguments.len() == 1 {
        for declaration in declarations(&fs::read(&arguments[0])?)? {
            println!(
                "{}\t{:?}\t{}\t{}",
                declaration.line, declaration.kind, declaration.name, declaration.signature
            );
        }
        return Ok(());
    }
    if arguments.len() != 5 || !matches!(arguments[0].to_str(), Some("--check" | "--write")) {
        return Err("usage: inventory FILE | inventory --check|--write RUST_HDL_ROOT GHDL_ROOT NVC_ROOT REPORT".into());
    }
    verify_repository(&arguments[1], RUST_HDL_REVISION)?;
    verify_repository(&arguments[2], GHDL_REVISION)?;
    verify_repository(&arguments[3], NVC_REVISION)?;
    let report = inventory_report(&arguments[1], &arguments[2], &arguments[3])?;
    if arguments[0] == Path::new("--check") {
        check_report(&fs::read_to_string(&arguments[4])?, &report)?;
        println!("PASS: every pinned declaration profile, owner, and reserved test ID matches the inventory");
    } else {
        fs::write(&arguments[4], report)?;
        println!("Generated {}", arguments[4].display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_overloads_aliases_defaults_and_generic_parameters() {
        let source = b"package inventory_probe is\n\
            generic (width : positive := 8);\n\
            type word is array (natural range <>) of bit;\n\
            subtype byte is word(7 downto 0);\n\
            constant initial : natural := 0;\n\
            function \"+\" (left, right : word) return word;\n\
            function \"+\" (left : word; right : natural := 1) return word;\n\
            alias add is \"+\" [word, word return word];\n\
            end package;\n";
        let declarations = declarations(source).unwrap();
        assert_eq!(declarations.len(), 7);
        let overloads: Vec<_> = declarations
            .iter()
            .filter(|declaration| declaration.kind == NodeKind::SubprogramDeclaration)
            .collect();
        assert_eq!(overloads.len(), 2);
        assert_ne!(overloads[0].signature, overloads[1].signature);
        assert!(overloads[1].signature.contains(":= 1"));
        assert_eq!(declarations[0].kind, NodeKind::GenericClause);
        assert_eq!(declarations[0].line, 2);
        assert_eq!(declarations[6].kind, NodeKind::AliasDeclaration);
    }

    #[test]
    fn normalizes_tokens_without_changing_literals() {
        let declarations = declarations(
            b"package probe is\n\
                            FUNCTION MixedCase (value : string := -- a comment\n\
                                \"AbC\") return string;\n\
                            end package;\n",
        )
        .unwrap();
        assert_eq!(declarations[0].name, "mixedcase");
        assert!(declarations[0].signature.contains("\"AbC\""));
        assert!(!declarations[0].signature.contains("comment"));
        assert!(!declarations[0].signature.contains('\n'));
    }

    #[test]
    fn rejects_package_body_inputs() {
        let error = declarations(b"package body probe is end package body;\n").unwrap_err();
        assert!(error.contains("bodies"));
    }

    #[test]
    fn does_not_inventory_malformed_declarations() {
        assert!(declarations(b"package broken is function\n").is_err());
    }

    #[test]
    fn selects_reference_2008_without_moving_source_lines() {
        let source = b"package probe is\n\
            --START-V08\n\
            function included return boolean;\n\
            --END-V08\n\
            --START-V19\n\
            function excluded return boolean;\n\
            --END-V19\n\
            function obsolete return boolean; --V87\n\
            end package;\n";
        let selected = select_2008(source).unwrap();
        assert_eq!(selected.len(), source.len());
        let declarations = declarations(&selected).unwrap();
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "included");
        assert_eq!(declarations[0].line, 3);
        assert!(select_2008(b"--START-V19\n").is_err());
        assert!(select_2008(b"--START-V42\n").is_err());
    }

    #[test]
    fn fails_closed_for_unassigned_functions_and_changed_reports() {
        let declaration = declarations(b"package probe is function surprise return bit; end;")
            .unwrap()
            .remove(0);
        assert!(owner("ieee.numeric_std", &declaration).is_err());
        assert!(owner("unknown.package", &declaration).is_err());
        assert!(check_report("one overload\ntwo overloads\n", "one overload\n").is_err());
        assert!(check_report("unchanged\n", "unchanged\n").is_ok());
    }

    #[test]
    fn keeps_2008_formatted_real_write_but_excludes_file_position_helpers() {
        let declarations = declarations(
            b"package probe is\n\
            procedure write (l : inout line; value : real; format : string);\n\
            procedure file_rewind (file f : text);\n\
            end;",
        )
        .unwrap();
        assert_eq!(
            owner("std.textio", &declarations[0]).unwrap(),
            (107, "text-io")
        );
        assert_eq!(
            owner("std.textio", &declarations[1]).unwrap(),
            (15, "excluded-2019")
        );
    }

    #[test]
    fn all_required_packages_and_contexts_have_declaration_only_source_paths() {
        assert_eq!(PACKAGES.len(), 18);
        for package in PACKAGES {
            assert!(!package.rust.contains("body"));
            assert!(!package.ghdl.unwrap_or_default().contains("body"));
            assert!(package.nvc.is_some());
            assert!(!package.nvc.unwrap_or_default().contains("body"));
        }
        assert!(!inline_code("<|`&").contains("<|"));
        assert!(!inline_code("<|`&").contains('|'));
    }
}
