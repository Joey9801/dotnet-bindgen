use crate::representations::{level_1::{self, Attribute}, level_2};

#[derive(Debug)]
pub struct LowerLevel2ToLevel1 {}


fn add_free_method(binding_method: &level_2::BindingMethod, methods: &mut Vec<level_1::Method>) {
    let dll_import_attr = level_1::Attribute {
        name: "DllImport".into(),
        args: vec![
            Box::new(level_1::Literal::String(binding_method.dll_name.to_string())),
        ],
    };

    let dll_import_args = binding_method
        .args
        .iter()
        .map(|a| level_1::MethodArg {
            name: a.source_ident().clone(),
            ty: a.dest_type().clone(),
        })
        .collect();

    let import_method = level_1::Method {
        attributes: vec![dll_import_attr],
        visibility: level_1::Visibility::Private,
        is_static: true,
        is_extern: true,
        name: binding_method.source_descriptor.thunk_name.clone().into(),
        return_type: level_1::CSharpType::Void,
        args: dll_import_args,
        body: None,
    };
    
    methods.push(import_method);
}

fn build_free_method_container(container: &level_2::MethodContainer) -> level_1::Object {
    let mut methods = Vec::new();
    for binding_method in &container.methods {
        add_free_method(binding_method, &mut methods);
    }
    
    level_1::Object {
        attributes: Vec::new(),
        visibility: level_1::Visibility::Public,
        is_sealed: false,
        is_static: true,
        kind: level_1::ObjectKind::Class,
        name: "FreeMethods".into(),
        methods,
        fields: Vec::new(),
    }
}

fn struct_field(f: &level_2::BindingField) -> level_1::ObjectField {
    let attr = level_1::Attribute {
        name: "FieldOffset".into(),
        args: vec![Box::new(level_1::Literal::Integer(f.offset as i32))],
    };
    
    level_1::ObjectField {
        attributes: vec![attr],
        visibility: level_1::Visibility::Public,
        ty: f.ty.clone(),
        name: f.name.clone().into(),
    }
}

fn build_struct_definition(s: &level_2::BindingStruct) -> level_1::Object {
    let attr = level_1::Attribute {
        name: "StructLayout".into(),
        args: vec![
            Box::new(level_1::Ident::new("LayoutKind.Explicit")),
            Box::new(level_1::Assignment::new(
                level_1::Ident::new("Size"),
                level_1::Literal::Integer(s.size as i32)
            )),
            Box::new(level_1::Assignment::new(
                level_1::Ident::new("Pack"),
                level_1::Literal::Integer(s.alignment as i32)
            )),
        ],
    };
    
    let fields = s.fields
        .iter()
        .map(struct_field)
        .collect();

    level_1::Object {
        attributes: vec![attr],
        visibility: level_1::Visibility::Public,
        is_sealed: false,
        is_static: false,
        kind: level_1::ObjectKind::Struct,
        name: s.name.clone().into(),
        fields,
        methods: Vec::new(),
    }
}

impl super::Pass for LowerLevel2ToLevel1 {
    type Input = level_2::BindingModule;
    type Output = level_1::CsSource;

    fn perform(&self, input: &Self::Input) -> Self::Output {
        let mut elements: Vec<Box<dyn level_1::TopLevelElement>> = Vec::new();

        elements.push(Box::new(level_1::UsingStatement {
            path: "System".to_string(),
        }));
        elements.push(Box::new(level_1::UsingStatement {
            path: "System.Runtime.InteropServices".to_string(),
        }));

        let mut namespace_content = Vec::<Box<dyn level_1::TopLevelElement>>::new();
        
        for s in &input.structs {
            namespace_content.push(Box::new(build_struct_definition(s)))
        }
        
        if let Some(free_methods) = &input.free_methods {
            namespace_content.push(Box::new(build_free_method_container(free_methods)));
        }

        elements.push(Box::new(level_1::Namespace {
            path: input.namespace.clone().into(),
            contents: namespace_content,
        }));

        level_1::CsSource { elements }
    }
}
