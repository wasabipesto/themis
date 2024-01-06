use actix_web::{get, middleware, web::Data, App, HttpResponse, HttpServer, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{pg::PgConnection, prelude::*};
use serde::Serialize;
use std::env::var;

mod helper;

// Diesel macro to get markets from the database table.
table! {
    market (id) {
        id -> Int4,
        title -> Varchar,
        platform -> Varchar,
        platform_id -> Varchar,
        url -> Varchar,
        open_days -> Float,
        volume_usd -> Float,
        prob_at_midpoint -> Float,
        prob_at_close -> Float,
        prob_time_weighted -> Float,
        resolution -> Float,
    }
}

#[derive(Debug, Queryable, Serialize, Selectable)]
#[diesel(table_name = market)]
pub struct Market {
    title: String,
    platform: String,
    platform_id: String,
    url: String,
    open_days: f32,
    volume_usd: f32,
    prob_at_midpoint: f32,
    prob_at_close: f32,
    prob_time_weighted: f32,
    resolution: f32,
}

#[derive(Debug, Serialize)]
pub struct Plot {
    platform_name: String,
    //platform_description: String,
    //platform_avatar_url: String,
    //brier_score: f32,
    x_series: Vec<f32>,
    y_series: Vec<f32>,
    //point_sizes: Vec<f32>,
    //point_descriptions: Vec<String>,
}

#[get("/calibration_plot")]
async fn calibration_plot(pool: Data<Pool<ConnectionManager<PgConnection>>>) -> impl Responder {
    let conn = &mut pool.get().expect("Failed to get connection from pool");
    let markets = market::table
        .filter(market::open_days.ge(0.0))
        .filter(market::volume_usd.ge(0.0))
        .select(Market::as_select())
        .load::<Market>(conn)
        .expect("Failed to query table.");

    let response = Plot {
        platform_name: "test".to_string(),
        x_series: Vec::from([0.0, 0.5, 1.0]),
        y_series: Vec::from([0.0, 0.5, 1.0]),
    };
    HttpResponse::Ok().json(response)
}

/// Server startup tasks.
#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    println!("Server starting...");
    let database_url =
        var("DATABASE_URL").expect("Required environment variable DATABASE_URL not set.");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool.");
    println!("Server started.");
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .wrap(middleware::Logger::default())
            .service(calibration_plot)
    })
    .bind(var("HTTP_BIND").unwrap_or(String::from("0.0.0.0:7041")))?
    .run()
    .await
}
