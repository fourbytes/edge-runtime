mod logger;

use anyhow::{anyhow, bail, Error};
use base::commands::start_server;
use base::server::WorkerEntrypoints;
use clap::builder::FalseyValueParser;
use clap::{arg, crate_version, value_parser, ArgAction, Command};
use deno_core::url::Url;
use sb_graph::emitter::EmitterFactory;
use sb_graph::import_map::load_import_map;
use sb_graph::{extract_from_file, generate_binary_eszip};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

fn cli() -> Command {
    Command::new("edge-runtime")
        .about("A server based on Deno runtime, capable of running JavaScript, TypeScript, and WASM services")
        .version(crate_version!())
        .arg_required_else_help(true)
        .arg(
            arg!(-v --verbose "Use verbose output")
                .conflicts_with("quiet")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(-q --quiet "Do not print any log messages")
                .conflicts_with("verbose")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(--"log-source" "Include source file and line in log messages")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("start")
                .about("Start the server")
                .arg(arg!(-i --ip <HOST> "Host IP address to listen on").default_value("0.0.0.0"))
                .arg(
                    arg!(-p --port <PORT> "Port to listen on")
                        .default_value("9000")
                        .value_parser(value_parser!(u16)),
                )
                .arg(arg!(--"main-service" <DIR> "Path to main service directory or eszip").default_value("examples/main"))
                .arg(arg!(--"disable-module-cache" "Disable using module cache").default_value("false").value_parser(FalseyValueParser::new()))
                .arg(arg!(--"import-map" <Path> "Path to import map file"))
                .arg(arg!(--"event-worker" <Path> "Path to event worker directory"))
                .arg(arg!(--"main-entrypoint" <Path> "Path to entrypoint in main service (only for eszips)"))
                .arg(arg!(--"events-entrypoint" <Path> "Path to entrypoint in events worker (only for eszips)"))
        )
        .subcommand(
            Command::new("bundle")
                .about("Creates an 'eszip' file that can be executed by the EdgeRuntime. Such file contains all the modules in contained in a single binary.")
                .arg(arg!(--"output" <DIR> "Path to output eszip file").default_value("bin.eszip"))
                .arg(arg!(--"entrypoint" <Path> "Path to entrypoint to bundle as an eszip").required(true))
                .arg(arg!(--"import-map" <Path> "Path to import map file"))
        ).subcommand(
        Command::new("unbundle")
            .about("Unbundles an .eszip file into the specified directory")
            .arg(arg!(--"output" <DIR> "Path to extract the ESZIP content").default_value("./"))
            .arg(arg!(--"eszip" <DIR> "Path of eszip to extract").required(true))
    )
}

//async fn exit_with_code(result: Result<(), Error>) {
//    match result {
//        Ok(()) => std::process::exit(0),
//        Err(error) => {
//            eprintln!("{:?}", error);
//            std::process::exit(1)
//        }
//    }
//}

fn main() -> Result<(), anyhow::Error> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // TODO: Tokio runtime shouldn't be needed here (Address later)
    let local = tokio::task::LocalSet::new();
    let res: Result<(), Error> = local.block_on(&runtime, async {
        let matches = cli().get_matches();

        if !matches.get_flag("quiet") {
            let verbose = matches.get_flag("verbose");
            let include_source = matches.get_flag("log-source");
            logger::init(verbose, include_source);
        }

        #[allow(clippy::single_match)]
        #[allow(clippy::arc_with_non_send_sync)]
        match matches.subcommand() {
            Some(("start", sub_matches)) => {
                let ip = sub_matches.get_one::<String>("ip").cloned().unwrap();
                let port = sub_matches.get_one::<u16>("port").copied().unwrap();

                let main_service_path = sub_matches
                    .get_one::<String>("main-service")
                    .cloned()
                    .unwrap();
                let import_map_path = sub_matches.get_one::<String>("import-map").cloned();
                let no_module_cache = sub_matches
                    .get_one::<bool>("disable-module-cache")
                    .cloned()
                    .unwrap();
                let event_service_manager_path =
                    sub_matches.get_one::<String>("event-worker").cloned();
                let maybe_main_entrypoint =
                    sub_matches.get_one::<String>("main-entrypoint").cloned();
                let maybe_events_entrypoint =
                    sub_matches.get_one::<String>("events-entrypoint").cloned();

                start_server(
                    ip.as_str(),
                    port,
                    main_service_path,
                    event_service_manager_path,
                    import_map_path,
                    no_module_cache,
                    None,
                    WorkerEntrypoints {
                        main: maybe_main_entrypoint,
                        events: maybe_events_entrypoint,
                    },
                )
                .await?;
            }
            Some(("bundle", sub_matches)) => {
                let output_path = sub_matches.get_one::<String>("output").cloned().unwrap();
                let import_map_path = sub_matches.get_one::<String>("import-map").cloned();

                let entry_point_path = sub_matches
                    .get_one::<String>("entrypoint")
                    .cloned()
                    .unwrap();

                let path = PathBuf::from(entry_point_path.as_str());
                if !path.exists() {
                    bail!("entrypoint path does not exist ({})", path.display());
                }

                let mut emitter_factory = EmitterFactory::new();
                let maybe_import_map = load_import_map(import_map_path.clone())
                    .map_err(|e| anyhow!("import map path is invalid ({})", e))?;
                let mut maybe_import_map_url = None;
                if maybe_import_map.is_some() {
                    let abs_import_map_path =
                        std::env::current_dir().map(|p| p.join(import_map_path.unwrap()))?;
                    maybe_import_map_url = Some(
                        Url::from_file_path(abs_import_map_path)
                            .map_err(|_| anyhow!("failed get import map url"))?
                            .to_string(),
                    );
                }
                emitter_factory.set_import_map(maybe_import_map.clone());

                let eszip = generate_binary_eszip(
                    path.canonicalize().unwrap(),
                    Arc::new(emitter_factory),
                    None,
                    maybe_import_map_url,
                )
                .await?;
                let bin = eszip.into_bytes();

                if output_path == "-" {
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();

                    handle.write_all(&bin)?
                } else {
                    let mut file = File::create(output_path.as_str())?;
                    file.write_all(&bin)?
                }
            }
            Some(("unbundle", sub_matches)) => {
                let output_path = sub_matches.get_one::<String>("output").cloned().unwrap();
                let eszip_path = sub_matches.get_one::<String>("eszip").cloned().unwrap();

                let output_path = PathBuf::from(output_path.as_str());
                let eszip_path = PathBuf::from(eszip_path.as_str());

                extract_from_file(eszip_path, output_path.clone()).await;

                println!(
                    "Eszip extracted successfully inside path {}",
                    output_path.to_str().unwrap()
                );
            }
            _ => {
                // unrecognized command
            }
        }
        Ok(())
    });

    res
}
