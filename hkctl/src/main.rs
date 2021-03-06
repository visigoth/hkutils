mod hkservice;
mod homes;
mod rooms;
mod zones;
mod accessories;
mod services;
mod service_groups;
mod action_sets;
mod triggers;
mod room;

use clap::{App, Arg, crate_version};
use tonic::transport::{Channel, Uri};
use tokio;
use hkservice::home_kit_service_client::HomeKitServiceClient;
use std::error::Error;

impl HomeKitServiceClient<Channel> {
    async fn create(host: &str, port: u32) -> Result<HomeKitServiceClient<Channel>, Box<dyn Error>> {
        let authority = format!("{}:{}", host, port);
        let endpoint = Uri::builder()
            .scheme("http")
            .authority(authority.as_str())
            .path_and_query("/")
            .build();
        let channel = tonic::transport::Channel::builder(endpoint.unwrap())
            .connect()
            .await?;
        Ok(HomeKitServiceClient::new(channel))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let room_opt = Arg::new("room")
        .long("room")
        .value_name("NAME OR UUID")
        .about("Room name pattern filter");
    let name_opt = Arg::new("name")
        .long("name")
        .value_name("NAME OR UUID")
        .about("Name pattern filter");
    let zone_opt = Arg::new("zone")
        .long("zone")
        .value_name("NAME OR UUID")
        .about("Zone name pattern filter");
    let name_arg = Arg::new("name")
        .value_name("NAME OR UUID")
        .about("Name");
    let operation_arg = Arg::new("operation")
        .value_name("OPERATION")
        .about("Operation to be performed")
        .possible_values(&["add", "remove"]);
    let mut app = App::new("hkctl")
        .version(crate_version!())
        .about("Command line porcelain for HomeKit")
        .arg(Arg::new("v")
             .short('v')
             .multiple(true)
             .about("Sets verbosity"))
        .arg(Arg::new("port")
             .long("port")
             .short('p')
             .value_name("PORT")
             .about("Local port to connect to"))
        .arg(Arg::new("home")
             .long("home")
             .about("Specify a home. Defaults to the primary home")
             .value_name("NAME OR UUID")
             .global(true))
        .subcommand(App::new("homes")
                    .about("Lists homes"))
        .subcommand(App::new("rooms")
                    .about("Lists rooms")
                    .arg(name_opt.clone()))
        .subcommand(App::new("zones")
                    .about("Lists zones")
                    .arg(room_opt.clone())
                    .arg(name_opt.clone()))
        .subcommand(App::new("accessories")
                    .about("Lists accessories")
                    .arg(room_opt.clone())
                    .arg(name_opt.clone())
                    .arg(zone_opt.clone()))
        .subcommand(App::new("services")
                    .about("Lists services")
                    .arg(name_opt.clone())
                    .arg(Arg::new("type")
                         .long("type")
                         .short('t')
                         .takes_value(true)
                         .multiple(true)))
        .subcommand(App::new("servicegroups")
                    .about("Lists service groups")
                    .arg(name_opt.clone()))
        .subcommand(App::new("services")
                    .about("Lists services")
                    .arg(name_opt.clone()))
        .subcommand(App::new("actionsets")
                    .about("Lists action sets")
                    .arg(name_opt.clone()))
        .subcommand(App::new("triggers")
                    .about("Lists triggers")
                    .arg(Arg::new("enabled_filter")
                         .about("Filter on enabled/disabled triggers")
                         .long("enabled")
                         .short('e')
                         .possible_values(&["either", "true", "false"]))
                    .arg(Arg::new("after")
                         .about("Triggered after specified time")
                         .long("after")
                         .short('a'))
                    .arg(Arg::new("before")
                         .about("Triggered before specified time")
                         .long("before")
                         .short('b'))
                    .arg(name_opt.clone()))
        .subcommand(App::new("room")
                    .about("Manipulate rooms")
                    .arg(operation_arg.clone().required(true))
                    .arg(name_arg.clone().required(true))
                    .arg(Arg::new("accessories")
                         .about("List of accessories to add/remove to/from a room. If empty, the room itself will be added or deleted")
                         .multiple(true)));

    let matches = app.get_matches_mut();
    let port = match matches.value_of_t::<u32>("port") {
        Ok(port) => port,
        Err(e) => {
            if e.kind == clap::ErrorKind::ArgumentNotFound {
                55123
            } else {
                e.exit()
            }
        }
    };
    let client = HomeKitServiceClient::create("127.0.0.1", port).await?;

    let subcommand_fn = matches.subcommand_name().map(|name| {
        match name {
            // Enumerate stuff
            "homes" => homes::run,
            "rooms" => rooms::run,
            "zones" => zones::run,
            "accessories" => accessories::run,
            "servicegroups" => service_groups::run,
            "services" => services::run,
            "actionsets" => action_sets::run,
            "triggers" => triggers::run,

            // Organize a home
            "room" => room::run,
            _ => panic!("Unrecognized subcommand name")
        }
    });

    if let Some(subcommand_fn) = subcommand_fn {
        let args = matches.subcommand_matches(matches.subcommand_name().unwrap()).unwrap();
        let result = subcommand_fn(args.clone(), client).await;
        if let Err(ref error) = result {
            match error.downcast_ref::<tonic::Status>() {
                Some(e) => {
                    println!("Error returned by server: {}", e);
                    return result
                },
                None => return result,
            }
        }
    } else {
        app.print_help()?;
    }
    Ok(())
}
