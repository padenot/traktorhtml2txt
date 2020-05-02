use chrono::NaiveTime;
use clap::*;
use kuchiki::traits::*;
use std::io::prelude::*;
use std::io::BufWriter;
use std::fs::File;

macro_rules! find_and_write {
    ($idx:ident, $fmt_ok:literal, $fmt_ko:literal, $outfile:ident, $tr: ident) => {
        if let Some(field) = $tr.as_node().children().nth($idx) {
            if !field.text_contents().is_empty() {
                write!($outfile, $fmt_ok, field.text_contents())?;
            } else {
                write!($outfile, $fmt_ko)?;
            }
        }
    };
}

fn main() -> Result<()> {
    let matches = clap_app!(
        traktorhtml2txt =>
            (version: "0.1")
            (author: "Paul Adenot <paul@paul.cx>")
            (about: "converts a tracktor html export to text")
            (@arg nostarttime: -n --notime "Don't output start time of tracks")
            (@arg input: +required "Sets the input file to use")
            (@arg output: "Sets the output file to use (stdout by default)")
    ).get_matches();
    let contents = match std::fs::read_to_string(matches.value_of("input").unwrap()) {
        Ok(string) => { string }
        Err(e) =>  {
            return Err(e.into());
        }
    };
    let mut outfile : Box<dyn Write>  = match matches.value_of("output") {
        Some(output_path) => match output_path {
            "-" => Box::new(BufWriter::new(std::io::stdout())),
            _ => Box::new(BufWriter::new(File::create(output_path)?)),
        }
        None => {
            Box::new(BufWriter::new(std::io::stdout()))
        }
    };
    let print_start_time = !matches.is_present("nostarttime");
    let document = kuchiki::parse_html().one(contents);
    let mut start_time: Option<NaiveTime> = None;
    let mut artist_idx = 0;
    let mut title_idx = 0;
    let mut release_idx = 0;
    let mut label_idx = 0;
    let mut starttime_idx = 0;

    let rows = document.select("tr").unwrap_or_else(|_| panic!("selector error, fix this."));

    for (i, tr) in rows.enumerate() {
        if i == 0 {
            for (j, column) in tr.as_node().children().enumerate() {
                match column.text_contents().as_ref() {
                    "Title" => { title_idx = j; }
                    "Artist" => { artist_idx = j; }
                    "Release" => { release_idx = j; }
                    "Label" => { label_idx = j; }
                    "Start Time" => { starttime_idx = j; }
                    _ => { }
                }
            }
            continue;
        }

        write!(outfile, "{:02}. ", i)?;

        if print_start_time && start_time.is_none() {
            if let Some(starttime) = tr.as_node().children().nth(starttime_idx) {
                if let Ok(start) = NaiveTime::parse_from_str(&starttime.text_contents(), "%Y/%m/%d %H:%M:%S") {
                    start_time = Some(start);
                }
            }
        }
        find_and_write!(artist_idx, "{} - ", "", outfile, tr);
        find_and_write!(title_idx, "{} - ", "", outfile, tr);
        find_and_write!(release_idx, "(from {}", "(", outfile, tr);
        find_and_write!(label_idx, " released on {})", ")", outfile, tr);
        if print_start_time {
            if let Some(starttime) = tr.as_node().children().nth(starttime_idx) {
                if !starttime.text_contents().is_empty() {
                    let t = starttime.text_contents();
                    if let Ok(start) = NaiveTime::parse_from_str(&t, "%Y/%m/%d %H:%M:%S") {
                        let offset = start.signed_duration_since(start_time.unwrap());
                        let mut mn = NaiveTime::from_hms_milli(0, 0, 0, 0);
                        mn += offset;
                        write!(outfile, " [{}]", mn)?;
                    }
                }
            }
        }
        writeln!(outfile)?;
    }
    Ok(())
}
