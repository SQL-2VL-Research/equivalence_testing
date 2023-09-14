use std::collections::HashMap;

use regex::Regex;
use smol_str::SmolStr;
use logos::{Filter, Lexer, Logos};

use super::{error::SyntaxError, subgraph_type::SubgraphType};

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeWithFields {
    Type(SubgraphType),  // no fields here for now
}

impl TypeWithFields {
    pub fn inner_ref(&self) -> &SubgraphType {
        match self {
            TypeWithFields::Type(tp) => tp,
        }
    }

    fn from_type_name(s: &str) -> Result<Self, SyntaxError> {
        Ok(Self::Type(SubgraphType::from_type_name(s)?))
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
    /// Used in function calls to be able to set the type list in handlers
    KnownList,
    /// Used in function calls to be able to set the type later. Similar to known, but indicates type compatibility.
    /// Will be extended in the future to link to the types call node which would supply the type
    Compatible,
    /// Used in function calls to pass the function's type constraints further
    PassThrough,
    /// Used in function calls to select multiple types among the allowed type list.
    /// Used in function declarations to specify the allowed type list, of which multiple can be selected.
    TypeListWithFields(Vec<TypeWithFields>),
}

/// splits a string with commas into a list of trimmed identifiers
pub fn get_identifier_names(name_list_str: SmolStr) -> Vec<SmolStr> {
    name_list_str
        .split(',')
        .map(|string| SmolStr::new(string.trim()))
        .collect::<Vec<_>>()
}

impl FunctionInputsType {
    /// tries to match a special type (any, known, compatible)
    fn try_extract_special_type(idents: &SmolStr) -> Option<FunctionInputsType> {
        match idents.as_str() {
            "any" => Some(FunctionInputsType::Any),
            "..." => Some(FunctionInputsType::PassThrough),
            "compatible" => Some(FunctionInputsType::Compatible),
            "known" => Some(FunctionInputsType::KnownList),
            _ => None,
        }
    }

    /// splits a string with commas into a list of trimmed identifiers
    fn parse_types(name_list_str: SmolStr) -> Result<Vec<TypeWithFields>, SyntaxError> {
        name_list_str
            .split(',')
            .map(|x| x.trim())
            .map(TypeWithFields::from_type_name)
            .collect::<Result<Vec<_>, _>>()
    }
}

/// This structure contains common fields for a node definition.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeCommon {
    /// Node identifier
    pub name: SmolStr,
    /// Type name if specified (basically an "on" modifier)
    pub type_name: Option<SubgraphType>,
    /// Graph modifier with mode if specified
    pub modifier: Option<(SmolStr, bool)>,
    /// Name of the call modifier if specified
    pub call_modifier_name: Option<SmolStr>,
    /// Name of the call modifier that this node affects, if specified
    pub affects_call_modifier_name: Option<SmolStr>,
    /// Name of the value that this node sets, if specified
    pub sets_value_name: Option<SmolStr>,
}

impl NodeCommon {
    pub fn with_name(name: SmolStr) -> Self {
        Self { name: name, type_name: None, modifier: None, call_modifier_name: None, affects_call_modifier_name: None, sets_value_name: None }
    }
}

/// a code unit represents a distinct command in code.
#[derive(Debug, Clone)]
pub enum CodeUnit {
    FunctionDeclaration(FunctionDeclaration),
    RegularNode {
        node_common: NodeCommon,
        literal: bool,
    },
    CallNode {
        node_common: NodeCommon,
        func_name: SmolStr,
        inputs: FunctionInputsType,
        modifiers: Option<Vec<SmolStr>>,
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
pub struct FunctionDeclaration {
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
    current_function_definition: Option<FunctionDeclaration>,
    call_ident_regex: Regex,
}

impl<'a> DotTokenizer<'a> {
    /// creates a DotTokenizer from a string with code
    pub fn from_str(source: &'a str) -> Self {
        DotTokenizer {
            lexer: DotToken::lexer(source),
            digraph_defined: false,
            current_function_definition: None,
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
                        Some(func) => {
                            if self.current_function_definition.is_some() {
                                break Some(Err(SyntaxError::new(
                                    format!("nested functional node definitions are not supported")
                                )))
                            } else {
                                self.current_function_definition = Some(func.clone());
                                break Some(Ok(CodeUnit::FunctionDeclaration(func)))
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
                                Some(DotToken::QuotedIdentifiers(idents)) => Some({
                                    let TypeWithFields::Type(tp) = return_some_err!(TypeWithFields::from_type_name(&idents));
                                    tp
                                }),
                                _ => None
                            };

                            if self.current_function_definition.is_none() {
                                return Some(Err(SyntaxError::new(format!(
                                    "Node definitions outside of subgraphs are not allowed"
                                ))))
                            }

                            let modifier = match node_spec.remove("MODIFIER") {
                                Some(DotToken::QuotedIdentifiers(idents)) => {
                                    let modifier_on = match node_spec.remove("MODIFIER_MODE") {
                                        Some(DotToken::QuotedIdentifiers(mode_name)) => match mode_name.as_str() {
                                            mode_name @ ("on" | "off") => Some(mode_name == "on"), _ => None
                                        }, _ => None,
                                    };
                                    let modifier_on = match modifier_on {
                                        Some(v) => v,
                                        None => break Some(Err(SyntaxError::new(format!(
                                            "Expected modifier_mode=\"on\" or modifier_mode=\"off\" after modifier=\"{idents}\" for node {node_name}"
                                        )))),
                                    };
                                    Some((idents, modifier_on))
                                },
                                _ => None
                            };

                            let call_modifier_name = match node_spec.remove("CALL_MODIFIER") {
                                Some(DotToken::QuotedIdentifiers(idents)) => {
                                    Some(idents)
                                },
                                _ => None,
                            };

                            let affects_call_modifier_name = match node_spec.remove("AFFECTS_CALL_MODIFIER") {
                                Some(DotToken::QuotedIdentifiers(idents)) => {
                                    Some(idents)
                                },
                                _ => None,
                            };

                            let sets_value_name = match node_spec.remove("SET_VALUE") {
                                Some(DotToken::QuotedIdentifiers(idents)) => {
                                    Some(idents)
                                },
                                _ => None,
                            };

                            let literal = match node_spec.remove("LITERAL") {
                                Some(DotToken::QuotedIdentifiers(idents)) => idents == SmolStr::new("t"),
                                _ => false
                            };

                            let node_common = NodeCommon {
                                name: node_name.clone(),
                                type_name,
                                modifier,
                                call_modifier_name,
                                affects_call_modifier_name,
                                sets_value_name,
                            };

                            if self.call_ident_regex.is_match(&node_name) {
                                let (
                                    input_type, modifiers, _,
                                ) = return_some_err!(
                                    parse_function_options(&node_name, node_spec, true)
                                );
                                if literal {
                                    break Some(Err(SyntaxError::new(format!(
                                        "call nodes can't be declaread as literals (node {node_name})"
                                    ))));
                                }
                                let func_name = SmolStr::new(node_name.split_once('_').unwrap().1);
                                break Some(Ok(CodeUnit::CallNode {
                                    node_common,
                                    func_name,
                                    inputs: input_type,
                                    modifiers,
                                }));
                            } else {
                                break Some(Ok(CodeUnit::RegularNode {
                                    node_common,
                                    literal,
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
                    if self.current_function_definition.is_some() {
                        self.current_function_definition = None;
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
fn try_parse_function_def(lex: &mut Lexer<'_, DotToken>) -> Result<Option<FunctionDeclaration>, SyntaxError> {
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
                            let (
                                input_type, modifiers, _,
                            ) = parse_function_options(
                                &node_name,
                                read_node_specification(lex, &node_name)?,
                                false,
                            )?;
                            let exit_node_name = parse_function_exit_node_name(lex, &node_name)?;
                            Ok(Some(FunctionDeclaration {
                                source_node_name: node_name,
                                exit_node_name,
                                input_type,
                                modifiers,
                            }))
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
    is_call_node: bool,
) -> Result<(FunctionInputsType, Option<Vec<SmolStr>>, Option<Vec<SubgraphType>>), SyntaxError> {
    let mut input_type = FunctionInputsType::None;
    let mut modifiers = Option::<Vec<SmolStr>>::None;
    let mut output_types = Option::<Vec<SubgraphType>>::None;
    if let Some(token) = node_spec.remove("TYPES") {
        if let DotToken::QuotedIdentifiersWithBrackets(idents) = token {
            if let Some(special_type) = FunctionInputsType::try_extract_special_type(&idents) {
                input_type = special_type;
            } else {
                input_type = FunctionInputsType::TypeListWithFields(
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
            let mods = get_identifier_names(value).into_iter()
                .filter(|el| el.len() > 0)
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
    if let Some(token) = node_spec.remove("OUT_TYPES") {
        if let DotToken::QuotedIdentifiersWithBrackets(value) = token {
            let out_types = get_identifier_names(value).into_iter()
                .filter(|el| el.len() > 0)
                .map(|el| SubgraphType::from_type_name(&el))
                .collect::<Result<Vec<_>, _>>()?;

            if out_types.len() > 0 {
                output_types = Some(out_types);
            }
        } else {
            return Err(SyntaxError::new(format!(
                "{node_name}[OUT_TYPES=... should take bracketed form: OUT_TYPES=\"[.., ]\""
            )));
        }
    }
    if is_call_node {
        if output_types.is_some() {
            return Err(SyntaxError::new(format!(
                "call nodes can't have a OUT_TYPES option, which is reserved for function definitions (node {node_name})"
            )));
        }
    }
    Ok((input_type, modifiers, output_types))
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
