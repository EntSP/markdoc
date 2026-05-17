use crate::types::*;
use std::collections::HashMap;

pub type FunctionImpl = fn(&[Scalar]) -> Result<Scalar>;

pub fn default_functions() -> HashMap<String, (ConfigFunction, FunctionImpl)> {
    let mut functions = HashMap::new();

    // equals function
    functions.insert(
        "equals".to_string(),
        (
            ConfigFunction {
                returns: Some(vec![ValidationType::Boolean]),
                parameters: None,
            },
            equals as FunctionImpl,
        ),
    );

    // and function
    functions.insert(
        "and".to_string(),
        (
            ConfigFunction {
                returns: Some(vec![ValidationType::Boolean]),
                parameters: None,
            },
            and as FunctionImpl,
        ),
    );

    // or function
    functions.insert(
        "or".to_string(),
        (
            ConfigFunction {
                returns: Some(vec![ValidationType::Boolean]),
                parameters: None,
            },
            or as FunctionImpl,
        ),
    );

    // not function
    functions.insert(
        "not".to_string(),
        (
            ConfigFunction {
                returns: Some(vec![ValidationType::Boolean]),
                parameters: None,
            },
            not as FunctionImpl,
        ),
    );

    functions
}

fn equals(args: &[Scalar]) -> Result<Scalar> {
    if args.len() != 2 {
        return Err(MarkdocError::TransformError(
            "equals requires exactly 2 arguments".to_string(),
        ));
    }

    Ok(Scalar::Boolean(scalar_eq(&args[0], &args[1])))
}

fn and(args: &[Scalar]) -> Result<Scalar> {
    for arg in args {
        if !truthy(arg) {
            return Ok(Scalar::Boolean(false));
        }
    }
    Ok(Scalar::Boolean(true))
}

fn or(args: &[Scalar]) -> Result<Scalar> {
    for arg in args {
        if truthy(arg) {
            return Ok(Scalar::Boolean(true));
        }
    }
    Ok(Scalar::Boolean(false))
}

fn not(args: &[Scalar]) -> Result<Scalar> {
    if args.len() != 1 {
        return Err(MarkdocError::TransformError(
            "not requires exactly 1 argument".to_string(),
        ));
    }

    Ok(Scalar::Boolean(!truthy(&args[0])))
}

fn truthy(value: &Scalar) -> bool {
    match value {
        Scalar::Null => false,
        Scalar::Boolean(b) => *b,
        Scalar::Number(n) => *n != 0.0,
        Scalar::String(s) => !s.is_empty(),
        Scalar::Array(a) => !a.is_empty(),
        Scalar::Object(o) => !o.is_empty(),
    }
}

fn scalar_eq(a: &Scalar, b: &Scalar) -> bool {
    match (a, b) {
        (Scalar::Null, Scalar::Null) => true,
        (Scalar::Boolean(a), Scalar::Boolean(b)) => a == b,
        (Scalar::Number(a), Scalar::Number(b)) => (a - b).abs() < f64::EPSILON,
        (Scalar::String(a), Scalar::String(b)) => a == b,
        _ => false,
    }
}
