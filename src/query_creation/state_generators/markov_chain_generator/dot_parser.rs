use std::{collections::HashMap, str::FromStr, fmt::Display};

use regex::Regex;
use smol_str::SmolStr;
use logos::{Filter, Lexer, Logos};
use sqlparser::ast::{DataType, ExactNumberInfo, ObjectName, Ident};

use super::error::SyntaxError;

#[derive(Logos, Debug, PartialEq)]
enum DotToken {
    #[regex(r"#[^\n]*", logos::skip)] // Skip # line comments
    #[regex(r"//[^\n]*", logos::skip)] // Skip // line comments
    #[regex(r"/\*", block_comment)] // Skip block comments
    #[regex(r"[^\S\n\r]+", logos::skip)] // Skip whitespace
    #[regex(r"[\n\r]+", logos::skip)] // Skip newline
    #[error]
    Error,

    #[regex(r"digraph")]
    Digraph,

    #[regex(r"subgraph")]
    Subgraph,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", identifier)]
    Identifier(SmolStr),

    #[regex(r#""[^"]*""#, quoted_identifier)]
    QuotedIdentifiers(SmolStr),

    #[regex(r#""\[[^"]*\]""#, quoted_identifier_with_brackets)]
    QuotedIdentifiersWithBrackets(SmolStr),

    #[regex(r"\{")]
    OpenDeclaration,

    #[regex(r"}")]
    CloseDeclaration,

    #[regex(r"\[")]
    OpenSpecification,

    #[regex(r"\]")]
    CloseSpecification,

    #[regex(r",")]
    Comma,

    #[regex(r"=")]
    Equals,

    #[regex(r"->")]
    Arrow,
}

/// extracts value from an identifier
fn identifier(lex: &mut Lexer<'_, DotToken>) -> SmolStr {
    SmolStr::new(lex.slice())
}

/// extracts value from an identifier with quotes
fn quoted_identifier(lex: &mut Lexer<'_, DotToken>) -> SmolStr {
    let quoted_ident = lex.slice();
    SmolStr::new(quoted_ident.get(1..quoted_ident.len() - 1).unwrap())
}

/// extracts value from an identifier with quotes and square brackets
fn quoted_identifier_with_brackets(lex: &mut Lexer<'_, DotToken>) -> SmolStr {
    let quoted_ident = lex.slice();
    let opening_bracket_pos = quoted_ident.find('[').unwrap();
    let closing_bracket_pos = quoted_ident.find(']').unwrap();
    SmolStr::new(
        quoted_ident
            .get(opening_bracket_pos+1..closing_bracket_pos)
            .unwrap(),
    )
}

/// callback that skips block comments
fn block_comment(lex: &mut Lexer<'_, DotToken>) -> Filter<()> {
    if let Some(end) = lex.remainder().find("*/") {
        lex.bump(end + 2);
        Filter::Skip
    } else {
        Filter::Emit(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubgraphType {
    Numeric,
    Val3,
    Array,
    ListExpr,
    String,
}

impl SubgraphType {
    pub fn get_all() -> Vec<SubgraphType> {
        vec![ Self::Numeric, Self::Val3, Self::Array, Self::ListExpr, Self::String ]
    }

    fn as_str(&self) -> &str {
        match self {
            SubgraphType::Numeric => "numeric",
            SubgraphType::Val3 => "3VL Value",
            SubgraphType::Array => "array",
            SubgraphType::ListExpr => "list expr",
            SubgraphType::String => "string",
        }
    }

    fn from_smolstr(smol_str: SmolStr) -> Result<Self, SyntaxError> {
        SubgraphType::from_str(smol_str.as_str())
    }

    pub fn to_data_type(&self) -> DataType {
        // TODO: this is a temporary solution
        // Select among types variants.
        match self {
            SubgraphType::Numeric => DataType::Numeric(ExactNumberInfo::None),
            SubgraphType::Val3 => DataType::Boolean,
            SubgraphType::Array => DataType::Array(Some(Box::new(DataType::Numeric(ExactNumberInfo::None)))),
            SubgraphType::ListExpr => DataType::Custom(ObjectName(vec![Ident::new("row_expression")]), vec![]),
            SubgraphType::String => DataType::String,
        }
    }
}

impl FromStr for SubgraphType {
    type Err = SyntaxError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "numeric" => Ok(SubgraphType::Numeric),
            "3VL Value" => Ok(SubgraphType::Val3),
            "array" => Ok(SubgraphType::Array),
            "list expr" => Ok(SubgraphType::ListExpr),
            "string" => Ok(SubgraphType::String),
            any => Err(SyntaxError::new(format!("Type {any} does not exist!")))
        }
    }
}

impl Display for SubgraphType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// this structure can be treated in two ways: either
/// as the output types the function should be restricted
/// to generate, when in the context of a call node, or
/// what input types the function supports, when mentioned
/// in function definition
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionInputsType {
    /// Used in function calls to choose all types out of the allowed type list
    Any,
    /// Used in function calls to choose none of the types out of the allowed type list
    /// Used in function declarations to indicate that no types are accepted.
    None,
    /// Used in function calls to be able to set the type variant later
    Known,
    /// Used in function calls to be able to set the type list later
    KnownList,
    /// Used in function calls to be able to set the type later. Similar to known, but indicates type compatibility.
    /// Will be extended in the future to link to the types call node which would supply the type
    Compatible,
    /// Used in function calls to pass the function's type constraints further
    PassThrough,
    /// Used in function calls to select a type among type variants.
    TypeName(SubgraphType),
    /// Used in function calls to select multiple types among the allowed type list.
    /// Used in function declarations to specify the allowed type list, of which multiple can be selected.
    TypeNameList(Vec<SubgraphType>),
    /// Used in function declarations to specify possible type variants, of which only one can be selected.
    TypeNameVariants(Vec<SubgraphType>),
}

/// splits a string with commas into a list of trimmed identifiers
fn get_identifier_names(name_list_str: SmolStr) -> Vec<SmolStr> {
    name_list_str
        .split(',')
        .map(|string| SmolStr::new(string.trim()))
        .collect::<Vec<_>>()
}

impl FunctionInputsType {
    /// tries to match a special type (any, known, compatible)
    fn try_extract_special_type(idents: &SmolStr, multiple: bool) -> Option<FunctionInputsType> {
        match idents.as_str() {
            "any" => Some(FunctionInputsType::Any),
            "..." => Some(FunctionInputsType::PassThrough),
            "compatible" => Some(FunctionInputsType::Compatible),
            "known" => Some(if multiple {FunctionInputsType::KnownList} else {FunctionInputsType::Known}),
            _ => None,
        }
    }

    /// splits a string with commas into a list of trimmed identifiers
    fn parse_types(name_list_str: SmolStr) -> Result<Vec<SubgraphType>, SyntaxError> {
        name_list_str
            .split(',')
            .map(|x| x.trim())
            .map(SubgraphType::from_str)
            .collect::<Result<Vec<_>, _>>()
    }
}

/// a code unit represents a distinct command in code.
#[derive(Debug)]
pub enum CodeUnit {
    Function(Function),
    NodeDef {
        name: SmolStr,
        literal: bool,
        type_name: Option<SubgraphType>,
        trigger: Option<(SmolStr, bool)>,
    },
    Call {
        node_name: SmolStr,
        func_name: SmolStr,
        inputs: FunctionInputsType,
        modifiers: Option<Vec<SmolStr>>,
        type_name: Option<SubgraphType>,
        trigger: Option<(SmolStr, bool)>,
    },
    Edge {
        node_name_from: SmolStr,
        node_name_to: SmolStr,
    },
    CloseDeclaration,
}

/// this structure contains function details extracted
/// from function declaration
#[derive(Clone, Debug)]
pub struct Function {
    pub source_node_name: SmolStr,
    pub exit_node_name: SmolStr,
    pub input_type: FunctionInputsType,
    pub modifiers: Option<Vec<SmolStr>>,
}

/// this structure stores the interral parser values needed
/// during code parsing
pub struct DotTokenizer<'a> {
    lexer: Lexer<'a, DotToken>,
    digraph_defined: bool,
    in_function_definition: bool,
    call_ident_regex: Regex,
}

impl<'a> DotTokenizer<'a> {
    /// creates a DotTokenizer from a string with code
    pub fn from_str(source: &'a str) -> Self {
        DotTokenizer {
            lexer: DotToken::lexer(source),
            digraph_defined: false,
            in_function_definition: false,
            call_ident_regex: Regex::new(r"^call[0-9]+_[A-Za-z_][A-Za-z0-9_]*$").unwrap(),
        }
    }
}

macro_rules! return_some_err {
    ($expr:expr) => {{
        match $expr {
            Ok(value) => value,
            Err(err) => break Some(Err(err)),
        }
    }};
}

impl<'a> Iterator for DotTokenizer<'a> {
    type Item = Result<CodeUnit, SyntaxError>;

    /// parse the tokens returned by lexer and return parsed code units one by one
    fn next(&mut self) -> Option<Self::Item> {
        let mut ignore_subgraph = false;
        loop {
            if ignore_subgraph {
                match self.lexer.next()? {
                    DotToken::OpenDeclaration => {
                        break Some(Err(SyntaxError::new(format!(
                            "Nested braces {{}} in subgraphs are not supported"
                        ))))
                    }
                    DotToken::CloseDeclaration => ignore_subgraph = false,
                    _ => continue,
                }
            }
            match self.lexer.next()? {
                DotToken::Error => continue,
                DotToken::Digraph => {
                    if self.digraph_defined {
                        break Some(Err(SyntaxError::new(
                            format!("multiple digraph definitions are not supported")
                        )))
                    }
                    return_some_err!(handle_digraph(&mut self.lexer));
                    self.digraph_defined = true;
                },
                DotToken::Subgraph => {
                    match return_some_err!(try_parse_function_def(&mut self.lexer)) {
                        Some(func @ CodeUnit::Function {..}) => {
                            if self.in_function_definition {
                                break Some(Err(SyntaxError::new(
                                    format!("nested functional node definitions are not supported")
                                )))
                            } else {
                                self.in_function_definition = true;
                                break Some(Ok(func))
                            }
                        },
                        _ => match self.lexer.next()? {
                            DotToken::OpenDeclaration => ignore_subgraph = true,
                            _ => break Some(Err(SyntaxError::new(
                                format!("Expected subgraph body: {}", self.lexer.slice())
                            )))
                        },
                    }
                },
                DotToken::Identifier(node_name) => {
                    match self.lexer.next() {
                        Some(DotToken::OpenSpecification) => {
                            let mut node_spec = return_some_err!(
                                read_opened_node_specification(&mut self.lexer, &node_name)
                            );
                            let type_name = match node_spec.remove("TYPE_NAME") {
                                Some(DotToken::QuotedIdentifiers(idents)) => Some(
                                    return_some_err!(SubgraphType::from_smolstr(idents))
                                ),
                                _ => None
                            };

                            let trigger = match node_spec.remove("TRIGGER") {
                                Some(DotToken::QuotedIdentifiers(idents)) => {
                                    let trigger_on = match node_spec.remove("TRIGGER_MODE") {
                                        Some(DotToken::QuotedIdentifiers(mode_name)) => match mode_name.as_str() {
                                            mode_name @ ("on" | "off") => Some(mode_name == "on"), _ => None
                                        }, _ => None,
                                    };
                                    let trigger_on = match trigger_on {
                                        Some(v) => v,
                                        None => break Some(Err(SyntaxError::new(format!(
                                            "Expected trigger_mode=\"on\" or trigger_mode=\"off\" after trigger=\"{idents}\" for node {node_name}"
                                        )))),
                                    };
                                    Some((idents, trigger_on))
                                },
                                _ => None
                            };
                            let literal = match node_spec.remove("LITERAL") {
                                Some(DotToken::QuotedIdentifiers(idents)) => idents == SmolStr::new("t"),
                                _ => false
                            };
                            if self.call_ident_regex.is_match(&node_name) {
                                let (input_type, modifiers) = return_some_err!(
                                    parse_function_options(&node_name, node_spec, true)
                                );
                                if literal {
                                    break Some(Err(SyntaxError::new(format!(
                                        "call nodes can't be declaread as literal (node {node_name})"
                                    ))));
                                }
                                break Some(Ok(CodeUnit::Call {
                                    node_name: node_name.clone(),
                                    func_name: SmolStr::new(node_name.split_once('_').unwrap().1),
                                    inputs: input_type,
                                    modifiers,
                                    type_name,
                                    trigger,
                                }));
                            } else {
                                break Some(Ok(CodeUnit::NodeDef {
                                    name: node_name,
                                    type_name,
                                    literal,
                                    trigger
                                }));
                            }
                        },
                        Some(DotToken::Arrow) => {
                            match self.lexer.next() {
                                Some(DotToken::Identifier(node_name_to)) => {
                                    break Some(Ok(CodeUnit::Edge {
                                        node_name_from: node_name, node_name_to
                                    }))
                                },
                                _ => break Some(Err(SyntaxError::new(
                                    format!("Expected an identifier after arrow: {node_name} -> ...")
                                )))
                            }
                        },
                        _ => break Some(Err(SyntaxError::new(
                            format!("Expected node definition ({node_name}[...]) or edge definition ({node_name} -> ...)")
                        )))
                    }
                },
                DotToken::QuotedIdentifiers(name) => break Some(Err(SyntaxError::new(
                    format!("Quoted identifiers are not supported: \"{name}\"")
                ))),
                DotToken::QuotedIdentifiersWithBrackets(name) => break Some(Err(SyntaxError::new(
                    format!("Quoted identifiers are not supported: \"{name}\"")
                ))),
                DotToken::CloseDeclaration => {
                    if self.in_function_definition {
                        self.in_function_definition = false;
                        break Some(Ok(CodeUnit::CloseDeclaration))
                    }
                },
                any => break Some(Err(SyntaxError::new(
                    format!("Unexpected {:?} : {}", any, self.lexer.slice())
                ))),
            }
        }
    }
}

/// handles a new subgraph definition
fn handle_digraph(lex: &mut Lexer<'_, DotToken>) -> Result<(), SyntaxError> {
    match lex.next() {
        None => Ok(()), // digraph at the end of file
        Some(token) => match token {
            DotToken::Identifier(_) => match lex.next() {
                Some(DotToken::OpenDeclaration) => Ok(()),
                _ => Err(SyntaxError::new(format!("expected digraph body"))),
            },
            DotToken::OpenDeclaration => Ok(()),
            _ => Err(SyntaxError::new(format!(
                "expected digraph identifier or digraph body"
            ))),
        },
    }
}

/// returns None if subgraph is not a function
fn try_parse_function_def(lex: &mut Lexer<'_, DotToken>) -> Result<Option<CodeUnit>, SyntaxError> {
    match lex.next() {
        Some(DotToken::Identifier(function_name)) => {
            if !function_name.starts_with("def_") {
                return Ok(None);
            }
            let function_name = match function_name.get(4..) {
                Some(decl_node_name) => decl_node_name,
                None => return Err(SyntaxError::new(format!("subgraph name def_ is forbidden"))),
            };
            match lex.next() {
                Some(DotToken::OpenDeclaration) => match lex.next() {
                    Some(DotToken::Identifier(node_name)) => {
                        if node_name != function_name {
                            Err(SyntaxError::new(
                                    format!(
                                        "Source node definition should be the first in the subgraph def_{} and should be named {}",
                                        function_name, function_name
                                    )
                                ))
                        } else {
                            let (input_type, modifiers) = parse_function_options(
                                &node_name,
                                read_node_specification(lex, &node_name)?,
                                false,
                            )?;
                            let exit_node_name = parse_function_exit_node_name(lex, &node_name)?;
                            Ok(Some(CodeUnit::Function(Function {
                                source_node_name: node_name,
                                exit_node_name,
                                input_type,
                                modifiers,
                            })))
                        }
                    }
                    _ => Err(SyntaxError::new(format!("expected source node definition"))),
                },
                _ => Err(SyntaxError::new(format!("expected subgraph body"))),
            }
        }
        Some(DotToken::OpenDeclaration) => Ok(None),
        _ => Err(SyntaxError::new(format!(
            "expected subgraph identifier or subgraph body"
        ))),
    }
}

/// parse function options, either in function declaration or in function call
fn parse_function_options(
    node_name: &SmolStr,
    mut node_spec: HashMap<SmolStr, DotToken>,
    call: bool,
) -> Result<(FunctionInputsType, Option<Vec<SmolStr>>), SyntaxError> {
    let mut input_type = FunctionInputsType::None;
    let mut modifiers = Option::<Vec<SmolStr>>::None;
    if let Some(token) = node_spec.remove("TYPE") {
        if let DotToken::QuotedIdentifiers(idents) = token {
            if let Some(special_type) = FunctionInputsType::try_extract_special_type(&idents, false) {
                input_type = special_type;
            } else {
                let idents = FunctionInputsType::parse_types(idents)?;
                if call {
                    match idents.len() {
                        1 => input_type = FunctionInputsType::TypeName(idents[0].clone()),
                        _ => {
                            return Err(SyntaxError::new(format!(
                                "call..._{node_name}[TYPE=... takes only 1 parameter"
                            )))
                        }
                    }
                } else {
                    input_type = FunctionInputsType::TypeNameVariants(idents);
                }
            }
        } else {
            return Err(SyntaxError::new(format!(
                "{node_name}[TYPE=... should take unbracketed form: TYPE=\".., \""
            )));
        }
    }
    if let Some(token) = node_spec.remove("TYPES") {
        if let DotToken::QuotedIdentifiersWithBrackets(idents) = token {
            if let Some(special_type) = FunctionInputsType::try_extract_special_type(&idents, true) {
                input_type = special_type;
            } else {
                input_type = FunctionInputsType::TypeNameList(
                    FunctionInputsType::parse_types(idents)?,
                );
            }
        } else {
            return Err(SyntaxError::new(format!(
                "{node_name}[TYPES=... should take bracketed form: TYPES=\"[.., ]\""
            )));
        }
    }
    if let Some(token) = node_spec.remove("MODS") {
        if let DotToken::QuotedIdentifiersWithBrackets(value) = token {
            let mods = get_identifier_names(value)
                .iter()
                .filter(|el| el.len() > 0)
                .map(|el| el.to_owned())
                .collect::<Vec<_>>();

            if mods.len() > 0 {
                modifiers = Some(mods);
            }
        } else {
            return Err(SyntaxError::new(format!(
                "{node_name}[MODS=... should take bracketed form: MODS=\"[.., ]\""
            )));
        }
    }
    Ok((input_type, modifiers))
}

/// expects and parses an exit node name after function declaration
fn parse_function_exit_node_name(
    lex: &mut Lexer<'_, DotToken>,
    node_name: &SmolStr,
) -> Result<SmolStr, SyntaxError> {
    let gen_exit_node_format_err = || {
        SyntaxError::new(format!(
            "Exit node should be defined after the {node_name} definition: EXIT_{node_name}[...]"
        ))
    };
    match lex.next() {
        Some(DotToken::Identifier(name)) => {
            if !name.starts_with("EXIT_") {
                Err(gen_exit_node_format_err())
            } else {
                read_node_specification(lex, &name)?;
                Ok(name)
            }
        }
        _ => Err(gen_exit_node_format_err()),
    }
}

/// read an opening bracket and a node specification afterwards
fn read_node_specification(
    lex: &mut Lexer<'_, DotToken>,
    node_name: &SmolStr,
) -> Result<HashMap<SmolStr, DotToken>, SyntaxError> {
    match lex.next() {
        Some(DotToken::OpenSpecification) => read_opened_node_specification(lex, node_name),
        _ => Err(SyntaxError::new(format!(
            "expected source node properties specifier {}[...]",
            node_name
        ))),
    }
}

/// read node specification after an opening bracket was read
fn read_opened_node_specification(
    lex: &mut Lexer<'_, DotToken>,
    node_name: &SmolStr,
) -> Result<HashMap<SmolStr, DotToken>, SyntaxError> {
    let mut props = HashMap::<_, _>::new();
    let gen_props_error = || {
        SyntaxError::new(format!(
            "expected source node property value: {}[PROP=\"VAL\", ...] or empty brackets: {}[]",
            node_name, node_name
        ))
    };
    loop {
        match lex.next() {
            Some(DotToken::Identifier(prop_name)) => match lex.next() {
                Some(DotToken::Equals) => match lex.next() {
                    Some(token @ DotToken::QuotedIdentifiersWithBrackets(_)) => {
                        props.insert(SmolStr::new(prop_name.to_uppercase()), token);
                    }
                    Some(token @ DotToken::QuotedIdentifiers(_)) => {
                        props.insert(SmolStr::new(prop_name.to_uppercase()), token);
                    }
                    Some(DotToken::Identifier(_)) => continue,
                    _ => break Err(gen_props_error()),
                },
                _ => break Err(gen_props_error()),
            },
            Some(DotToken::CloseSpecification) => break Ok(props),
            Some(DotToken::Comma) => continue,
            _ => break Err(gen_props_error()),
        }
    }
}
