use crate::representations::level_0::*;

/// Inject formatting tokens (newlines, indents, etc..) with a basic heuristic
/// to make the token stream more human-readable.
#[derive(Debug)]
pub struct FormatLevel0 { }

fn insert_newlines(old_stream: &TokenStream) -> TokenStream {
    let mut new_stream = TokenStream::new();
    
    for tree in old_stream.iter() {
        match tree {
            TokenTree::Group(Group { delimiter, content}) => {
                let mut formatted_content = insert_newlines(content);
                match delimiter {
                    Delimiter::Brace => {
                        new_stream.push(Formatting::Newline);
                        formatted_content.prepend(Formatting::Newline);
                        new_stream.push(Group {
                            delimiter: delimiter.clone(),
                            content: formatted_content,
                        });
                        new_stream.push(Formatting::Newline);
                    },
                    _ => new_stream.push(Group {
                        delimiter: delimiter.clone(),
                        content: formatted_content,
                    }),
                }
            }
            TokenTree::Punct(Punct::Semicolon) => {
                new_stream.push(tree.clone());
                new_stream.push(Formatting::Newline);
            }
            _ => new_stream.push(tree.clone()),
        }
    }
    
    new_stream
}

fn insert_indents(old_stream: &TokenStream, indent_level: usize) -> TokenStream {
    let mut new_stream = TokenStream::new();
    
    for tree in old_stream.iter() {
        match tree {
            TokenTree::Group(Group { delimiter, content}) => {
                let mut formatted_content = insert_indents(content, indent_level + 1);
                
                // This condition should always be true.
                // It sets the indentation before the closing brace to be equal
                // the indentation of the opening brace.
                if let Some(TokenTree::Formatting(Formatting::Indent(ref mut x))) = formatted_content.iter_mut().last() {
                    *x -= 1;
                }

                new_stream.push(Group {
                    delimiter: delimiter.clone(),
                    content: formatted_content,
                });
            }
            TokenTree::Formatting(Formatting::Newline) => {
                new_stream.push(Formatting::Newline);
                new_stream.push(Formatting::Indent(indent_level));
            }
            _ => new_stream.push(tree.clone()),
        }
    }
    
    new_stream
}


impl super::Pass for FormatLevel0 {
    type Input = TokenStream;
    type Output = TokenStream;

    fn perform(&self, input: &Self::Input) -> Self::Output {
        let output = insert_newlines(input);
        let output = insert_indents(&output, 0);
        
        output
    }
}