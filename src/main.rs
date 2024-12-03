use actix_web::body::Body;
use actix_web::dev::ServiceResponse;
use actix_web::http::{header::*, StatusCode};
use actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result};
use serde::{Deserialize, Serialize};

use std::io::Write;
use std::time::{Duration, Instant, SystemTime};

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

macro_rules! render {
    ($template:path) => (Render(|o| $template(o)));
    ($template:path, $($arg:expr),*) => {{
        Render(|o| $template(o, $($arg),*))
    }};
    ($template:path, $($arg:expr),* ,) => {{
        Render(|o| $template(o, $($arg),*))
    }};
}

pub struct Render<T: FnOnce(&mut dyn Write) -> std::io::Result<()>>(pub T);

impl<T: FnOnce(&mut dyn Write) -> std::io::Result<()>> From<Render<T>> for actix_web::body::Body {
    fn from(t: Render<T>) -> Self {
        let mut buf = Vec::new();
        match t.0(&mut buf) {
            Ok(()) => buf.into(),
            Err(_e) => "Render failed".into(),
        }
    }
}

async fn index() -> impl Responder {
    HttpResponse::Ok().body(render!(crate::templates::home_html))
}

#[derive(Deserialize)]
struct Query {
    debate: String,
}

async fn debate(query: web::Query<Query>) -> impl Responder {
    HttpResponse::Ok().body(render!(
        templates::debate_html,
        query.debate.clone(),
        vec!["Cuillère."]
    ))
}

#[derive(Serialize, Deserialize)]
struct APIResponse {
    debate: String,
    elapsed_time: u128,
    answer: String,
}

async fn api(query: actix_web::Result<web::Query<Query>>) -> impl Responder {
    match query {
        Ok(query) => {
            let start = Instant::now();
            let answer = String::from("Cuillère.");
            let duration = start.elapsed();

            let res = APIResponse {
                debate: query.debate.to_owned(),
                elapsed_time: duration.as_micros(),
                answer,
            };

            let res_text = serde_json::to_string(&res).unwrap();

            HttpResponse::Ok()
                .header(CONTENT_TYPE, "application/json; charset=utf-8")
                .body(res_text)
        }
        Err(_) => {
            HttpResponse::BadRequest()
                .header(CONTENT_TYPE, "application/json; charset=utf-8")
                .body(serde_json::json!({
                    "error": "Passez moi un query param \"debate\" pour que je puisse faire quelque chose"
                }))
        }
    }
}

fn render_404(res: ServiceResponse<Body>) -> Result<ErrorHandlerResponse<Body>> {
    let mut res = res;
    res.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(mime::TEXT_HTML_UTF_8.as_ref()).unwrap(),
    );
    Ok(ErrorHandlerResponse::Response(res.map_body(
        |_head, _body| {
            actix_web::dev::ResponseBody::Body(
                render!(
                    templates::error_html,
                    "Page introuvable",
                    "On a cherché partout (oui, même sous le canapé), et on n'arrive pas à trouver cette page. Déso."
                ).into()
            )
        },
    )))
}

fn render_500(res: ServiceResponse<Body>) -> Result<ErrorHandlerResponse<Body>> {
    let mut res = res;
    res.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(mime::TEXT_HTML_UTF_8.as_ref()).unwrap(),
    );
    Ok(ErrorHandlerResponse::Response(res.map_body(
        |_head, _body| {
            actix_web::dev::ResponseBody::Body(
                render!(
                    templates::error_html,
                    "Erreur interne du serveur",
                    "On a tout cassé, mais vous inquiétez pas, on va tout réparer."
                )
                .into(),
            )
        },
    )))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::NOT_FOUND, render_404)
                    .handler(StatusCode::INTERNAL_SERVER_ERROR, render_500),
            )
            .route("/static/{filename}", web::get().to(static_file))
            .route("/", web::get().to(index))
            .route("/debate", web::get().to(debate))
            .route("/api", web::get().to(api))
    })
    .bind("127.0.0.1:7878")?
    .run()
    .await
}

fn static_file(path: web::Path<(String,)>) -> HttpResponse {
    use crate::templates::statics::StaticFile;
    let name = &path.0;
    if let Some(data) = StaticFile::get(name) {
        let far_expires = SystemTime::now() + FAR;
        HttpResponse::Ok()
            .set(Expires(far_expires.into()))
            .set(ContentType(data.mime.clone()))
            .body(data.content)
    } else {
        HttpResponse::NotFound()
            .reason("No such static file.")
            .finish()
    }
}

/// A duration to add to current time for a far expires header.
static FAR: Duration = Duration::from_secs(180 * 24 * 60 * 60);
