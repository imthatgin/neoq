use std::iter::Peekable;

use proc_macro2::{Delimiter, Group, Literal, TokenStream, TokenTree};
use quote::quote;
use syn::Expr;


pub(crate) fn generate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream = TokenStream::from(input).into_iter().peekable();
    
    let body = generate_body(stream);
    let expanded = quote! {{
        let mut __q = ::std::string::String::new();
        #body
        __q
    }};

    proc_macro::TokenStream::from(expanded)
}

fn generate_body<I>(mut stream: Peekable<I>) -> TokenStream
where I: Iterator<Item = TokenTree> {
    let mut buf: Vec<TokenTree> = Vec::new();
    let mut output = TokenStream::new();

    while let Some(tree) = stream.peek() {
        match tree {
            TokenTree::Ident(ident) if ident == "if" => {
                flush_buf(&mut buf, &mut output);
                stream.next();

                let condition = parse_until_brace(&mut stream).expect("failed to parse `if` condition");
                let block = take_balanced_group(&mut stream, Delimiter::Brace).expect("failed to parse `if` inner block");
                let inner = generate_body(block.stream().into_iter().peekable());

                output.extend(quote! {
                    if #condition {
                        #inner
                    }
                });
            },
            _ => buf.push(stream.next().unwrap())
        }
    }

    flush_buf(&mut buf, &mut output);
    output
}

fn parse_until_brace<I>(stream: &mut Peekable<I>) -> Option<Expr>
where I: Iterator<Item = TokenTree> {
    let mut output = TokenStream::new();
    let mut nest = 0usize;

    while let Some(tree) = stream.peek() {
        match tree {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace && nest == 0 => break,
            TokenTree::Punct(punct) if punct.as_char() == '(' || punct.as_char() == '[' => {
                nest += 1;
                output.extend(stream.next());
            },
            TokenTree::Punct(punct) if punct.as_char() == ')' || punct.as_char() == ']' => {
                nest = nest.saturating_sub(1);
                output.extend(stream.next());
            }
            _ => output.extend(stream.next()),
        }
    }

    syn::parse2(output).ok()
}

fn flush_buf(buf: &mut Vec<TokenTree>, out: &mut TokenStream) {
    if buf.is_empty() {
        return;
    }

    let literal = fragment_to_literal(buf);

    out.extend(quote! {
        __q.push_str(#literal);
        __q.push_str("\n");
    });
    buf.clear();
}

fn fragment_to_literal(fragment: &[TokenTree]) -> Literal {
    let mut s = String::new();
    let mut prev_end: Option<(usize, usize)> = None;

    for tree in fragment {
        let text = tree.to_string();
        let span = tree.span();
        let start = span.start();

        if let Some((prev_line, prev_col)) = prev_end {
            if start.line > prev_line {
                s.push('\n');
            } else if start.column > prev_col {
                s.push(' ');
            }
        }

        s.push_str(&text);
        let end = span.end();
        prev_end = Some((end.line, end.column));
    }

    Literal::string(&s)
}

fn take_balanced_group<I>(stream: &mut Peekable<I>, delim: Delimiter) -> Option<Group> 
where I: Iterator<Item = TokenTree> {
    match stream.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == delim => Some(group),
        _ => None,
    }
}