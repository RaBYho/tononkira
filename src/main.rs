use clap::{Arg, Command};
use colored_json::ToColoredJson;
use tabled::{Style, Table};
use tononkira::{colorizer::print_colorized, parser::Parser, types::Options};

fn cli() -> Command<'static> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    Command::new("Tononkira")
        .version(VERSION)
        .author("Tsiry Sandratraina <tsiry.sndr@aol.com>")
        .about(
            r#"
 _______                      _    _           
|__   __|                    | |  (_)          
   | | ___  _ __   ___  _ __ | | ___ _ __ __ _ 
   | |/ _ \| '_ \ / _ \| '_ \| |/ / | '__/ _` |
   | | (_) | | | | (_) | | | |   <| | | | (_| |
   |_|\___/|_| |_|\___/|_| |_|_|\_\_|_|  \__,_|
                                                
Search lyrics from tononkira.serasera.org
"#,
        )
        .arg(
            Arg::new("artist")
                .short('a')
                .long("artist")
                .help("song's artist")
                .takes_value(true)
                .multiple_values(true)
                .required(false),
        )
        .arg(
            Arg::new("title")
                .short('t')
                .long("title")
                .help("song's title")
                .takes_value(true)
                .multiple_values(true)
                .required(false),
        )
        .arg(
            Arg::new("lyrics")
                .short('l')
                .long("lyrics")
                .help("song's lyrics")
                .takes_value(true)
                .multiple_values(true)
                .required(false),
        )
        .arg(
            Arg::new("json")
                .short('j')
                .long("json")
                .help("output in json format")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::new("keywords")
                .help("The song's title, artist or URL")
                .takes_value(true)
                .multiple_values(true)
                .required(false),
        )
}

#[tokio::main]
async fn main() -> Result<(), surf::Error> {
    let matches = cli().get_matches();

    let artist = matches
        .values_of("artist")
        .map(|vals| vals.collect::<Vec<_>>().join(" "));
    let title = matches
        .values_of("title")
        .map(|vals| vals.collect::<Vec<_>>().join(" "));
    let lyrics_param = matches
        .values_of("lyrics")
        .map(|vals| vals.collect::<Vec<_>>().join(" "));

    let is_artist_search = artist.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
    let is_title_search = title.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
    let is_lyrics_search = lyrics_param.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);

    let keywords = matches
        .values_of("keywords")
        .map(|vals| vals.collect::<Vec<_>>().join(" "))
        .unwrap_or_default();

    let options = Options {
        is_artist_search,
        is_title_search,
        is_lyrics_search,
        artist,
        title,
        lyrics: lyrics_param,
    };

    let parser = Parser::new();
    let lyrics = parser.search(&keywords, options).await?;

    if lyrics.is_empty() {
        println!("No lyrics found");
        return Ok(());
    }

    if matches.is_present("json") {
        println!(
            "{}",
            serde_json::to_string(&lyrics)?.to_colored_json_auto()?
        );
    } else if lyrics.len() == 1 {
        print_colorized(&lyrics[0]);
    } else {
        println!("\n{}", Table::new(&lyrics).with(Style::psql()));
        println!("\n💡 Astuce: Pour afficher les paroles d'une chanson, tapez :");
        println!("   tononkira -a \"<artiste>\" -t \"<titre>\"");
    }

    Ok(())
}
