use apollo_router::plugin::Plugin;
use apollo_router::plugin::PluginInit;
use apollo_router::{graphql, register_plugin};
use apollo_router::services::execution;
use apollo_router::services::subgraph;
use apollo_router::services::supergraph;
use schemars::JsonSchema;
use serde::Deserialize;
use tower::BoxError;
use tower::ServiceBuilder;
use tower::ServiceExt;
use std::collections::HashMap;
use apollo_parser::Parser;
use std::iter::once;
use std::ops::ControlFlow;
use std::sync::Arc;
use graphql_parser::{query as q, parse_query, parse_schema, schema as s, schema};
use graphql_parser::query::{Definition, Document, OperationDefinition, Type, TypeCondition};
use std::vec::Vec;
use apollo_router::layers::ServiceBuilderExt;
use http::StatusCode;


#[derive(Debug)]
struct HelloWorld {
    #[allow(dead_code)]
    configuration: Conf,
    supergraph_sdl: Arc<String>
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct Conf {
    // Put your plugin configuration here. It will automatically be deserialized from JSON.
    name: String, // The name of the entity you'd like to say hello to
    allow_list: HashMap<String,Vec<String>>
}

// This is a bare bones plugin that can be duplicated when creating your own.
#[async_trait::async_trait]
impl Plugin for HelloWorld {
    type Config = Conf;

    async fn new(init: PluginInit<Self::Config>) -> Result<Self, BoxError> {
        Ok(HelloWorld {
            configuration: init.config,
            supergraph_sdl: init.supergraph_sdl
        })
    }

    fn supergraph_service(&self, service: supergraph::BoxService) -> supergraph::BoxService {

        let supergraph_sdl = self.supergraph_sdl.clone().to_string();

        let allow_list = self.configuration.allow_list.clone();

        let sdl = supergraph_sdl.to_owned();

        fn convert_map<'a>(map: &'a HashMap<String, Vec<String>>) -> HashMap<&'a str, Vec<&'a str>> {
            let mut result = HashMap::new();
            for (key, value) in map.iter() {
                let new_key: &str = key.as_str();
                let new_value = value.iter().map(|s| s.as_str()).collect();
                result.insert(new_key, new_value);
            }
            result
        }

        // Say hello when our service is added to the router_service
        // stage of the router plugin pipeline.
        // #[cfg(test)]
        // println!("Hello {}", self.configuration.name);

        // Always use service builder to compose your plugins.
        // It provides off the shelf building blocks for your plugin.
        ServiceBuilder::new()
            .checkpoint(move |req: supergraph::Request| {
                let schema_ast = parse_schema::<&str>(&*supergraph_sdl).unwrap();

                let inc_query = req.supergraph_request.body().query.as_ref().unwrap();
                let claims = req.context.get("apollo_authentication::JWT::claims");
                if let Ok(Some(claims)) = claims {
                    match claims {
                        serde_json::Value::Object(map) => {
                            for (key, value) in map.iter() {
                                println!("{}: {}", key, value);
                            }
                        }
                        _ => {
                            println!("Unexpected value found in `claims`.");
                        }
                    }
                }

                //println!("{:#?}",claims);


                let query_ast = parse_query::<&str>(&*inc_query).unwrap();

                // check if role = user.
                // if so, then we run schema validator.


                let valid = schema_validator(query_ast,schema_ast.clone(),convert_map(&allow_list.clone()));

                if !valid {
                    // let's log the error
                    tracing::error!("Operation is not allowed!");

                    // Prepare an HTTP 400 response with a GraphQL error message
                    let res = supergraph::Response::error_builder()
                        .error(
                            graphql::Error::builder()
                                .message("invalid query")
                                .extension_code("ANONYMOUS_OPERATION")
                                .build(),
                        )
                        .status_code(StatusCode::BAD_REQUEST)
                        .context(req.context)
                        .build()?;
                    Ok(ControlFlow::Break(res))
                } else {
                    // we're good to go!
                    tracing::info!("Operation is allowed!");
                    Ok(ControlFlow::Continue(req))
                }
            })
            // .rate_limit()
            // .checkpoint()
            // .timeout()
            .service(service)
            .boxed()
    }

    fn execution_service(&self, service: execution::BoxService) -> execution::BoxService {
        //This is the default implementation and does not modify the default service.
        // The trait also has this implementation, and we just provide it here for illustration.
        service
    }

    // Called for each subgraph
    fn subgraph_service(&self, _name: &str, service: subgraph::BoxService) -> subgraph::BoxService {
        // Always use service builder to compose your plugins.
        // It provides off the shelf building blocks for your plugin.
        ServiceBuilder::new()
            // .map_request()
            // .map_response()
            // .rate_limit()
            // .checkpoint()
            // .timeout()
            .service(service)
            .boxed()
    }
}


fn get_field_type<'a>(field_name: &str, schema_doc: s::Document<'a,&'a str>, parent_type: &str) -> Option<String> {
    for definition in &schema_doc.definitions {
        if let s::Definition::TypeDefinition(def) = definition {
            if let s::TypeDefinition::Object(obj) = &def {
                if obj.name == parent_type {
                    for field in &obj.fields {
                        if field.name == field_name {
                            return match &field.field_type {
                                Type::NamedType(n_type) => {
                                    Some(n_type.to_string())
                                }
                                Type::ListType(l_type) => {
                                    Some(l_type.to_string())
                                }
                                Type::NonNullType(nn_type) => {
                                    Some(nn_type.to_string())
                                }
                            };
                        }
                    }
                }
            }
        }
    }
    None
}

fn check_fields_allowed<'a>(field_map: &HashMap<&str, Vec<&str>>, sel_set: &Vec<q::Selection<'a,&'a str>>, parent_type: &str, schema_doc: s::Document<'a,&'a str>) -> bool {
    for sel in sel_set {
        match sel {
            q::Selection::Field(field) => {
                let field_name = field.name;
                if field_name == "__typename" {
                    return true
                }
                if let Some(allowed_fields) = field_map.get(parent_type) {
                    if allowed_fields.contains(&"*") {
                        return true;
                    }
                    if !allowed_fields.contains(&field_name) {
                        return false;
                    }
                    if let Some(field_type) = get_field_type(field_name, schema_doc.clone(), parent_type) {
                        if let sub_sel_set = &field.selection_set.items {
                            if !check_fields_allowed(field_map, sub_sel_set, &field_type, schema_doc.clone()) {
                                return false;
                            }
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            },
            q::Selection::FragmentSpread(spread) => {
                if let fragment_type = spread.fragment_name {
                    // if !check_fields_allowed(field_map, &spread.fragment.selection_set.items, fragment_type, schema_doc) {
                    //     return false;
                    // }
                }
            },
            q::Selection::InlineFragment(frag) => {
                if let Some(fragment_type) = &frag.type_condition {
                    if !check_fields_allowed(field_map, &frag.selection_set.items, fragment_type.to_string().as_str(), schema_doc.clone()) {
                        return false;
                    }
                } else {
                    // current parent type is used for inline fragment without type condition
                    if !check_fields_allowed(field_map, &frag.selection_set.items, parent_type, schema_doc.clone()) {
                        return false;
                    }
                }
            },
        }
    }
    true
}

fn schema_validator<'a>(query_ast: Document<'a,&'a str>, schema_ast: s::Document<'a,&'a str>, allow_list: HashMap<&str, Vec<&str>>) -> bool {

    let schema_doc = schema_ast.clone();
    let mut object_types = vec![];

    for definition in &schema_ast.definitions {
        match definition {
            s::Definition::TypeDefinition(type_definition) => {
                match type_definition {
                    s::TypeDefinition::Object(obj_type) => {
                        object_types.push(obj_type);
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    let allowed = allow_list;
    let definitions = &query_ast.definitions;

    // loop over all the operations in the query
    for definition in definitions {
        match definition {
            Definition::Operation(op) => {
                match op {
                    OperationDefinition::SelectionSet(sset) => {
                        let selects = &sset.items;
                        let parent_type = "Query"; // The root query type
                        let result = check_fields_allowed(&allowed, selects, &parent_type,schema_doc.clone());
                        println!("validation result is {result}");
                        if result {
                            println!("Bad query");
                            return result
                        }
                    }
                    OperationDefinition::Query(qq) => {
                        let selects = &qq.selection_set.items;
                        let parent_type = "Query"; // The root query type
                        let result = check_fields_allowed(&allowed, selects, &parent_type,schema_doc.clone());
                        println!("validation result is {result}");
                        if !result {
                            println!("Bad query");
                            return result
                        }
                    }
                    OperationDefinition::Mutation(mt) => {
                        let selects = &qq.selection_set.items;
                        let parent_type = "Mutation"; // The root query type
                        let result = check_fields_allowed(&allowed, selects, &parent_type,schema_doc.clone());
                        println!("validation result is {result}");
                        if !result {
                            println!("Bad query");
                            return result
                        }
                    }
                    OperationDefinition::Subscription(_) => {}
                }
            }
            Definition::Fragment(frag) => {
                if let fragment_type = &frag.type_condition {
                    match fragment_type {
                        TypeCondition::On(ty) => {
                            if !check_fields_allowed(&allowed, &frag.selection_set.items, ty, schema_doc.clone()) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }



true
}


// This macro allows us to use it in our plugin registry!
// register_plugin takes a group name, and a plugin name.
//
// In order to keep the plugin names consistent,
// we use using the `Reverse domain name notation`
register_plugin!("example", "hello_world", HelloWorld);




#[cfg(test)]
mod tests {
    // If we run this test as follows: cargo test -- --nocapture
    // we will see the message "Hello Bob" printed to standard out
    #[tokio::test]
    async fn display_message() {
        let config = serde_json::json!({
            "plugins": {
                "example.hello_world": {
                    "name": "Bob"
                }
            }
        });
        // Build a test harness. Usually we'd use this and send requests to
        // it, but in this case it's enough to build the harness to see our
        // output when our service registers.
        let _test_harness = apollo_router::TestHarness::builder()
            .configuration_json(config)
            .unwrap()
            .build_router()
            .await
            .unwrap();
    }
}
