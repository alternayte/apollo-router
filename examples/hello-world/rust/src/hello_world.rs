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
use std::ops::{ControlFlow, Deref};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use graphql_parser::{query as q, parse_query, parse_schema, schema as s, schema};
use graphql_parser::query::{Definition, Document, OperationDefinition, Type, TypeCondition};
use std::vec::Vec;
use apollo_router::layers::ServiceBuilderExt;
use http::StatusCode;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;
use serde_json::{json, Value};
// use crate::plugins::authentication::APOLLO_AUTHENTICATION_JWT_CLAIMS;




#[derive(Debug)]
struct HelloWorld<'a> {
    #[allow(dead_code)]
    configuration: Conf,
    supergraph_sdl: Arc<String>,
    //supergraph_sdl: Arc<Mutex<Option<Arc<Arc<String>>>>>,
    // schema_ast: s::Document<'a,&'a str>,
    //schema_ast: Option<s::Document<'a,&'a str>>,
    schema_ast: Option<s::Document<'a, String>>,
    schema_hash: HashMap<String,String>
}

// impl HelloWorld {
//     fn get_schema_ast(self) {
//        let pp = self.supergraph_sdl.clone();
//     }
// }

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct Conf {
    // Put your plugin configuration here. It will automatically be deserialized from JSON.
    name: String, // The name of the entity you'd like to say hello to
    allow_list: HashMap<String,Vec<String>>
}

// This is a bare bones plugin that can be duplicated when creating your own.
#[async_trait::async_trait]
impl Plugin for HelloWorld<'static> {
    type Config = Conf;

    async fn new(init: PluginInit<Self::Config>) -> Result<Self, BoxError> {
        let mut schema_doc: Option<s::Document<&str>> = None;

        // Initialize the supergraph_sdl when it becomes available
        // For example, if it's injected into the `init` parameter:
        let sdl = init.supergraph_sdl.clone();
        //*supergraph_sdl.lock().unwrap() = Some(Arc::new(sdl));


        //let supergraph_sdl = init.supergraph_sdl.clone().to_string();

        //let super_sdl_guard = supergraph_sdl.lock().unwrap();
        //let sdl_str = super_sdl_guard.as_ref().unwrap().as_str();
        // let sdl = super_sdl.clone();
        // let sdl_str: String = sdl.into();
        //
        // let schema_doc = parser(sdl_str);
        let doc_a = parse_schema(&sdl);
        let doc_b = doc_a?.into_static();
        drop(sdl);
        //let schema_doc = parse_schema::<&str>(&sdl)?.to_owned();
        let schema_doc = doc_b;
        //let my_schema = Some();
        // let my_schema = schema_doc.clone();
        // let defs = my_schema.definitions;
        //let sdl = String::from(supergraph_sdl);






        Ok(HelloWorld {
            configuration: init.config,
            supergraph_sdl:Default::default(),
            schema_hash: HashMap::new(),
            schema_ast: Some(schema_doc)
        })
    }



    fn supergraph_service(&self, service: supergraph::BoxService) -> supergraph::BoxService {

        let mut valid_query: bool = false;


        let supergraph_sdl = self.supergraph_sdl.clone();

        let allow_list = self.configuration.allow_list.clone();

        let sdl = supergraph_sdl.clone();; //.to_owned().clone();

        let schema_doc_opt =self.schema_ast.clone();
        let schema_doc = schema_doc_opt.clone().unwrap();

        //let schema_ast = get_schema_ast(&sdl);

        if self.schema_ast.is_some() {
            println!("get schema and cache!");
            //let schema_ast = parse_schema::<&str>(&*supergraph_sdl).unwrap();
            //self.schema_ast = schema_ast

        }

        //
        fn convert_map<'a>(map: &'a HashMap<String, Vec<String>>) -> HashMap<&'a str, Vec<&'a str>> {
            let mut result = HashMap::new();
            for (key, value) in map.iter() {
                let new_key: &str = key.as_str();
                let new_value = value.iter().map(|s| s.as_str()).collect();
                result.insert(new_key, new_value);
            }
            result
        }

        println!("super service");


        // Say hello when our service is added to the router_service
        // stage of the router plugin pipeline.
        // #[cfg(test)]
        // println!("Hello {}", self.configuration.name);

        // Always use service builder to compose your plugins.
        // It provides off the shelf building blocks for your plugin.
        ServiceBuilder::new()
            .checkpoint(move |req: supergraph::Request| {
                let start_time = Instant::now();

                let mut is_user: bool = false;
                let mut valid: bool = false;

                let elapsed_time = start_time.elapsed();

                println!("Elapsed time bef: {:?}", elapsed_time);

                //let inc_query = req.supergraph_request..body().query.as_ref().unwrap();
                let claims = req.context.get("apollo_authentication::JWT::claims");
                //let request_schema = schema_ast.clone();
                let inc_query = req.supergraph_request.body().query.as_ref().unwrap().to_owned().clone();

                let mut current_roles:Vec<String> = Vec::new();
                if let Ok(Some(claims)) = claims {
                    match claims {
                        serde_json::Value::Object(map) => {
                            for (key, value) in map.iter() {
                                println!("{}: {}", key, value);


                                match key.as_str() {
                                    "https://graph.frontiers-ss-dev.info/jwt/claims" | "https://graph.test-frontiersin.net/jwt/claims" | "https://graph.frontiersin.org/jwt/claims" => {
                                        let roles = json!(value);
                                        println!("my roles {}",roles);
                                        if let Some(obj) = roles.as_object() {
                                            for (key, value) in obj {
                                                match key.as_str() {
                                                    "x-hasura-allowed-roles" => {
                                                        if let Some(array) = value.as_array() {
                                                            for role in array {
                                                                if let Some(role_str) = role.as_str() {
                                                                    current_roles.push(role_str.to_owned());
                                                                }
                                                            }
                                                        }
                                                    }
                                                    "x-allowed-roles" => {
                                                        if let Some(array) = value.as_array() {
                                                            for role in array {
                                                                if let Some(role_str) = role.as_str() {
                                                                    current_roles.push(role_str.to_owned());
                                                                }
                                                            }
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                println!("Key: {}", key);
                                                println!("Value: {}", value);
                                            }
                                        }
                                    }
                                    "ext" => {
                                        let graph_obj = json!(value);
                                        if let Some(obj) = graph_obj.as_object() {
                                            for (key, value) in obj {
                                                extract_role_from_claims(key, value, &mut current_roles);

                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {
                            println!("Unexpected value found in `claims`.");
                        }
                    }

                }
                if current_roles.contains(&String::from("user")) {
                    is_user = true;
                }

                if !is_user {
                    let elapsed_time = start_time.elapsed();
                    println!("Elapsed time non user: {:?}", elapsed_time);

                    Ok(ControlFlow::Continue(req))
                } else {

                    // let key = String::from("schema_key");
                    // let mut hasher = self.schema_hash.clone();
                    // let value = match hasher.entry(key.clone()) {
                    //     std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                    //     std::collections::hash_map::Entry::Vacant(entry) => {
                    //         let new_value = parse_schema::<&str>(r#""#).unwrap();
                    //         entry.insert(new_value)
                    //     }
                    // };

                    // let ss = self.schema_ast.clone().unwrap();
                    //
                    // //let schema_ast = parse_schema::<&str>(&*supergraph_sdl).unwrap();
                    let inc_q = inc_query.clone();
                    //
                    let query_ast = parse_query::<&str>(&inc_q).unwrap();
                    // let pp = ss.clone();
                    let pp = schema_doc.clone();


                    //drop(ss);

                    valid = schema_validator(query_ast,pp,convert_map(&allow_list.clone()));

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
                        let elapsed_time = start_time.elapsed();
                        println!("Elapsed time: {:?}", elapsed_time);
                        Ok(ControlFlow::Break(res))
                    } else {
                        // we're good to go!
                        tracing::info!("Operation is allowed!");
                        let elapsed_time = start_time.elapsed();
                        println!("Elapsed time: {:?}", elapsed_time);
                        Ok(ControlFlow::Continue(req))
                    }
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

// fn get_schema_ast_2(sdl: &str) -> s::Document<String> {
//     let sdll = self.supergraph_sdl.clone();
//     let schema_ast = parse_schema::<String>(sdl).unwrap().clone();
//     schema_ast
// }

// fn parser(sdl: String) -> s::Document<'static, String> {
//     let my_sdl = sdl.clone();
//     let schema = parse_schema(&my_sdl).unwrap();
//     let pp = schema.clone();
//     pp.into()
// }
fn extract_role_from_claims(key: &String, value: &Value, current_roles: &mut Vec<String>) {
    match key.as_str() {
        "https://graph.frontiers-ss-dev.info/jwt/claims" | "https://graph.test-frontiersin.net/jwt/claims" | "https://graph.frontiersin.org/jwt/claims" => {
            let roles = json!(value);
            println!("my roles {}",roles);
            if let Some(obj) = roles.as_object() {
                for (key, value) in obj {
                    match key.as_str() {
                        "x-hasura-allowed-roles" => {
                            if let Some(array) = value.as_array() {
                                for role in array {
                                    if let Some(role_str) = role.as_str() {
                                        current_roles.push(role_str.to_owned());
                                    }
                                }
                            }
                        }
                        "x-allowed-roles" => {
                            if let Some(array) = value.as_array() {
                                for role in array {
                                    if let Some(role_str) = role.as_str() {
                                        current_roles.push(role_str.to_owned());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    println!("Key: {}", key);
                    println!("Value: {}", value);
                }
            }
        }
        _ => {}
    }
}
fn get_field_type<'a>(field_name: &str, schema_doc: &s::Document<String>, parent_type: &str) -> Option<String> {
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

fn check_fields_allowed<'a>(field_map: &HashMap<&str, Vec<&str>>, sel_set: &Vec<q::Selection<'a,&'a str>>, parent_type: &str, schema_doc: &s::Document<String>) -> bool {
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
                    if let Some(mut field_type) = get_field_type(field_name, &schema_doc.clone(), parent_type) {
                        field_type = field_type.replace("[","").replace("]","").replace("!","");
                        if let sub_sel_set = &field.selection_set.items {
                            if !check_fields_allowed(field_map, sub_sel_set, &field_type, &schema_doc.clone()) {
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
                    if !check_fields_allowed(field_map, &frag.selection_set.items, fragment_type.to_string().as_str(), &schema_doc.clone()) {
                        return false;
                    }
                } else {
                    // current parent type is used for inline fragment without type condition
                    if !check_fields_allowed(field_map, &frag.selection_set.items, parent_type, &schema_doc.clone()) {
                        return false;
                    }
                }
            },
        }
    }
    true
}

fn schema_validator<'a>(query_ast: Document<'a,&'a str>, schema_ast: s::Document<String>, allow_list: HashMap<&str, Vec<&str>>) -> bool {

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
                        let result = check_fields_allowed(&allowed, selects, &parent_type,&schema_doc.clone());
                        println!("validation result is {result}");
                        if result {
                            println!("Bad query");
                            return result
                        }
                    }
                    OperationDefinition::Query(qq) => {
                        let selects = &qq.selection_set.items;
                        let parent_type = "Query"; // The root query type
                        let result = check_fields_allowed(&allowed, selects, &parent_type,&schema_doc.clone());
                        println!("validation result is {result}");
                        if !result {
                            println!("Bad query");
                            return result
                        }
                    }
                    OperationDefinition::Mutation(mt) => {
                        let selects = &mt.selection_set.items;
                        let parent_type = "Mutation"; // The root query type
                        let result = check_fields_allowed(&allowed, selects, &parent_type,&schema_doc.clone());
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
                            if !check_fields_allowed(&allowed, &frag.selection_set.items, ty, &schema_doc.clone()) {
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
