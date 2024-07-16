use actix_web::{
    middleware,
    web::{self, Data},
    App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use std::{env, sync::RwLock};

use juniper::{graphql_object, EmptySubscription, FieldResult};
use juniper_actix::{graphiql_handler, graphql_handler};

struct Context {
    // To allow for race-proof mutable access to name
    name: RwLock<String>,
}

impl juniper::Context for Context {}

struct Query;
struct Mutation;

#[graphql_object]
#[graphql(context = Context)]
impl Query {
    fn hello_world(world: String) -> String {
        format!("Hello {world}!")
    }
    fn hello(context: &Context) -> FieldResult<String> {
        Ok(format!("Hello {0}!", context.name.read().unwrap()))
    }
}

#[graphql_object]
#[graphql(context = Context)]
impl Mutation {
    fn set_name(name: String, context: &Context) -> String {
        let mut guard = context.name.write().unwrap();
        *guard = name;
        "Nice name!".to_string()
    }
}

type Schema = juniper::RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

fn schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::<Context>::new())
}

async fn graphiql() -> Result<HttpResponse, Error> {
    graphiql_handler("/graphql", Some("/subscriptions")).await
}

async fn graphql(
    req: HttpRequest,
    payload: web::Payload,
    schema: Data<Schema>,
) -> Result<HttpResponse, Error> {
    let context = Context {
        name: RwLock::new("Unknown".to_owned()),
    };
    graphql_handler(&schema, &context, req, payload).await
}

async fn homepage() -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("content-type", "text/html"))
        .message_body(
            "<html><h1>juniper_actix/subscription example</h1>\
                   <div>visit <a href=\"/graphiql\">GraphiQL</a></div>\
             </html>",
        )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(schema()))
            .wrap(middleware::Logger::default())
            .service(
                web::resource("/graphql")
                    .route(web::post().to(graphql))
                    .route(web::get().to(graphql)),
            )
            .service(web::resource("/graphiql").route(web::get().to(graphiql)))
            .default_service(web::to(homepage))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
