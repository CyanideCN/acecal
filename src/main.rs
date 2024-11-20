use clap::{CommandFactory, Parser};
use glob::glob;
use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};

#[derive(Default, Debug)]
struct PerBasinACE {
    wpac: i32,
    nio: i32,
    shem: i32,
    epac: i32,
    atl: i32,
}

impl PerBasinACE {
    fn sum(&self) -> i32 {
        self.to_array().iter().sum()
    }

    fn to_array(&self) -> [i32; 5] {
        [self.wpac, self.nio, self.shem, self.epac, self.atl]
    }

    fn update_ace(&mut self, basin: &Basin, ace: i32) {
        match basin {
            Basin::WPAC => self.wpac += ace,
            Basin::EPAC => self.epac += ace,
            Basin::NIO => self.nio += ace,
            Basin::SHEM => self.shem += ace,
            Basin::ATL => self.atl += ace,
        }
    }

    fn basin_count(&self) -> i32 {
        let mut count: i32 = 0;
        for var in self.to_array() {
            if var > 0 {
                count += 1;
            }
        }
        count
    }

    fn summarize(&self, separator: &str) -> String {
        let mut text = "".to_string();
        if self.wpac > 0 {
            text += &format!("WPAC: {:.4}", self.wpac as f32 / 10000.);
            text += separator;
        }
        if self.epac > 0 {
            text += &format!("ECPAC: {:.4}", self.epac as f32 / 10000.);
            text += separator;
        }
        if self.atl > 0 {
            text += &format!("ATL: {:.4}", self.atl as f32 / 10000.);
            text += separator;
        }
        if self.shem > 0 {
            text += &format!("SHEM: {:.4}", self.shem as f32 / 10000.);
            text += separator;
        }
        if self.nio > 0 {
            text += &format!("NIO: {:.4}", self.nio as f32 / 10000.);
        }
        text.strip_suffix(separator).unwrap_or(&text).to_string()
    }

    fn print_perbasin_ace(&self) {
        print!("     Per basin ACE: ");
        print!("{}\n", self.summarize("  "));
    }
}

#[derive(Default)]
struct StormStats {
    atcf_code: String,
    max_wind: i32,
    ace: PerBasinACE,
}

enum Basin {
    WPAC,
    NIO,
    SHEM,
    EPAC,
    ATL,
}

fn is_tropical(storm_type: &str) -> bool {
    let non_tropical = ["SD", "SS", "LO", "MD", "EX", "DB", "ET"];
    // `ET` added for special cases
    return !non_tropical.contains(&storm_type);
}

fn is_synop_time(time_str: &str) -> bool {
    let t: i32 = time_str.parse().unwrap();
    return (t % 6) == 0;
}

fn get_basin(latitude: f32, longitude: f32) -> Basin {
    if latitude < 0. {
        return Basin::SHEM;
    }
    if longitude < 100. {
        if latitude < 40. {
            return Basin::NIO;
        } else {
            if longitude < 70.0 {
                return Basin::ATL;
            } else {
                return Basin::WPAC;
            }
        }
    } else if longitude <= 180. {
        return Basin::WPAC;
    } else {
        if longitude < 240. {
            return Basin::EPAC;
        } else if longitude > 300. {
            return Basin::ATL;
        } else {
            // Complex boundary between EPAC and NATL, return EPAC for now.
            return Basin::EPAC;
        }
    }
    //panic!("Incorrect coordinates");
}

fn print_ace(ace_map: HashMap<i32, PerBasinACE>) {
    println!("{}", "--------Summary--------");
    for year in ace_map.keys() {
        let tmp = ace_map.get(year).unwrap();
        if tmp.sum() > 0 {
            println!("{}: ", year);
            println!("{}", tmp.summarize("\n"));
        }
    }
}

fn process_bdeck_files(file_list: Vec<String>) -> (Vec<StormStats>, HashMap<i32, PerBasinACE>) {
    let mut yearly_ace_map: HashMap<i32, PerBasinACE> = HashMap::new();
    let mut storm_stats: Vec<StormStats> = Vec::new();
    for file_path in file_list {
        let file = fs::read_to_string(file_path).unwrap();
        let mut last_time = "";
        let atcf_basin = &file[0..2];
        let atcf_number = &file[4..6];
        let atcf_code = atcf_basin.to_owned() + atcf_number;
        let mut ss_tmp = StormStats::default();
        ss_tmp.atcf_code = atcf_code;
        for line in file.lines() {
            let line_time = &line[8..18];
            if last_time == line_time {
                continue;
            }
            last_time = line_time;
            let mut year: i32 = (&line_time[..4]).parse().unwrap();
            // Handle southern hemisphere
            let month: i32 = (&line_time[4..6]).parse().unwrap();
            if (month > 6) & (atcf_basin == "SH") {
                year += 1;
            }
            if !yearly_ace_map.contains_key(&year) {
                yearly_ace_map.insert(year, PerBasinACE::default());
            }
            let line_len = line.len() - 1;
            let temp_wind: &str;
            if line_len < 51 {
                // Fix case that a space is missing in short-style bdeck
                temp_wind = &line[line_len - 3..];
            } else {
                temp_wind = &line[48..51];
            }
            let mut wind: i32 = temp_wind
                .strip_prefix(" ")
                .unwrap_or(temp_wind)
                .parse()
                .unwrap_or(0);
            if wind == 999 {
                wind = 0;
            }
            if wind > ss_tmp.max_wind {
                ss_tmp.max_wind = wind;
            }
            let lat_str = &line[35..39];
            let lat_string: String = lat_str[..3]
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            let mut latitude: f32 = lat_string.parse::<f32>().unwrap() / 10.;
            if &lat_str[3..4] == "S" {
                latitude *= -1.
            }
            let lon_str = &line[41..46];
            let lon_string: String = lon_str[..4]
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            let mut longitude: f32 = lon_string.parse::<f32>().unwrap() / 10.;
            if &lon_str[4..5] == "W" {
                longitude = 360. - longitude;
            }
            let mut storm_type = "";
            if line_len > 59 {
                // Read type of storm in long-style bdeck
                storm_type = &line[59..61];
            }
            if is_tropical(storm_type) & is_synop_time(&line_time[8..10]) {
                if wind >= 35 {
                    let basin = get_basin(latitude, longitude);
                    let ace = wind.pow(2);
                    ss_tmp.ace.update_ace(&basin, ace);
                    let tmp = yearly_ace_map.get_mut(&year).unwrap();
                    tmp.update_ace(&basin, ace);
                }
            }
        }
        storm_stats.push(ss_tmp);
    }
    (storm_stats, yearly_ace_map)
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(value_name = "FILE")]
    input_file: Option<String>,

    #[arg(short = 'd', long, value_name = "DIR", help = "Directory or pattern")]
    input_dir: Option<String>,
}

fn list_files(path: String) -> Result<Vec<String>, Error> {
    let file_list: Vec<String>;
    let md = fs::metadata(&path);
    match md {
        Ok(md) => {
            if md.is_dir() {
                file_list = fs::read_dir(path.strip_prefix(" ").unwrap_or(path.as_str()))
                    .unwrap()
                    .map(|res| res.unwrap().path().display().to_string())
                    .collect();
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Not a directory: {}", path),
                ));
            }
        }
        Err(e) => {
            let g = glob(&path);
            match g {
                Ok(g) => file_list = g.map(|path| path.unwrap().display().to_string()).collect(),
                Err(_) => return Err(e),
            }
        }
    }
    Ok(file_list)
}

fn main() {
    let args = Cli::parse();
    let file_list: Vec<String>;
    if let Some(file) = args.input_file {
        file_list = vec![file];
    } else {
        if let Some(dir) = args.input_dir {
            let files = list_files(dir);
            match files {
                Ok(files) => file_list = files,
                Err(e) => {
                    println!("{}", e);
                    Cli::command().print_help().unwrap();
                    return;
                }
            }
        } else {
            Cli::command().print_help().unwrap();
            return;
        }
    }
    if file_list.len() == 0 {
        println!("No files found!");
        return;
    }
    let (storm_stats, ace_map) = process_bdeck_files(file_list);
    for ss in storm_stats {
        println!(
            "{}: {:7.4}   Max Wind: {:3}kt",
            ss.atcf_code,
            ss.ace.sum() as f32 / 10000.,
            ss.max_wind
        );
        if ss.ace.basin_count() > 1 {
            ss.ace.print_perbasin_ace();
        }
    }
    print_ace(ace_map);
}
