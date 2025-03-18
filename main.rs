use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, disable_help_flag = true)]

struct Args {
    #[arg(long)]
    help: bool,
    #[command(subcommand)]
    command: Option<Commands>, //comanda care are subcomenzile lui
}

#[derive(Subcommand, Debug)]
enum Commands {
    Split {
        file: String, //positional argument

        #[arg(short, long, default_value = "1b")]
        size: String,
    },
    Unsplit {
        file: String, //positional
    },
}

fn chunk(size: &str) -> Result<usize, io::Error> {
    let size_lower = size.trim().to_lowercase();
    let mut suffix = String::new(); //suflixul
    let mut nr_char = String::new();

    for c in size_lower.chars() {
        if c.is_ascii_digit() || c == '.' {
            nr_char.push(c);
        } else {
            suffix.push(c);
        }
    }

    //in cazul in care am dat -s k/m/g
    if nr_char.is_empty() {
        nr_char = "1".to_string();
    }

    //transformarea numarului in f64
    let result = nr_char.as_str().parse::<f64>(); //convert to f64
    let nr = match result {
        Ok(v) => v,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Eroare la convertire in f64",
            ))
        }
    };

    //calcularea de bytes in functie de sufixul dat
    let bytes = match suffix.as_str() {
        "" => nr.round() as usize,
        "b" => nr.round() as usize,
        "kb" | "k" => (nr * 1024.0).round() as usize,
        "mb" | "m" => (nr * 1024.0 * 1024.0).round() as usize,
        "gb" | "g" => (nr * 1024.0 * 1024.0 * 1024.0).round() as usize,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Sufix necunoscut",
            ))
        }
    };

    //am pus o limita de 2g
    const MAX_SIZE: usize = 2 * 1024 * 1024 * 1024;
    if bytes > MAX_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Size prea mare"));
    }
    println!("Cat trebuie sa aiba un fisier split {}", bytes);
    Ok(bytes)
}

fn take_file_from_path(file: &String) -> Result<&str, io::Error> {
    //SE PREIA DOAR FISIERUL FARA CALEA SA
    let path = Path::new(file);
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Fisier inexistent",
        ));
    }
    match path.file_name() {
        Some(res1) => match res1.to_str() {
            Some(res2) => Ok(res2),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Error at unwrap",
            )),
        },
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Error at unwrap",
        )),
    }
}

fn hash(buffer: Vec<u8>, file_split: &str) -> Result<(), io::Error> {
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let result = hasher.finalize();

    let hash_file_name = format!("{}.hash", file_split);
    let mut hash_file = File::create(hash_file_name)?;
    hash_file.write_all(&result)?;
    Ok(())
}

fn split(file: &String, s: usize) -> Result<(), io::Error> {
    let mut chunk = vec![0; s];
    let mut ct = 1;

    let file_split = take_file_from_path(file)?;
    println!("Numele fișierului extras: {}", file_split);

    //fac un hash pe fisierul original pentru a compara la unsplit
    let buffer = std::fs::read(file)?;
    hash(buffer, file_split)?;

    //deschidem fisierul
    let mut to_read = File::open(file)?;

    //am creat un folder pentru fiecare fisier split
    let s = format!("{}_parts_splitted", file_split);

    //unde vom salva partile fisierului
    let parts_splitted = Path::new(&s);
    if parts_splitted.exists() {
        fs::remove_dir_all(parts_splitted)?;
        fs::create_dir_all(parts_splitted)?;
    } else {
        fs::create_dir_all(parts_splitted)?;
    }

    loop {
        let bytes_read = to_read.read(&mut chunk)?;
        if bytes_read == 0 {
            break;
        }

        //daca se depaseste limita de 4 cifre
        if ct >= 10_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Numărul de părți depășește limita de 4 cifre.",
            ));
        }

        // se creează numele fisierului
        let f = format!("{}.part{:04}.split", file_split, ct);

        // se creează fisierul și se adaugă în directorul parts_splitted
        let mut part_file = File::create(parts_splitted.join(f))?;

        // se scrie în fisier bufferul
        part_file.write_all(&chunk[0..bytes_read])?;

        ct += 1;
    }
    //sterg fisierul original

    //fs::remove_file(file)?;

    Ok(())
}

fn take_number_split(dir: &Path) -> Result<i32, io::Error> {
    let ct = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".split"))
        .count();
    Ok(ct as i32)
}

fn take_size(dir: &Path, file_name: &str, nr_total: i32) -> Result<(usize, usize), io::Error> {
    let mut ct = 1;
    let mut size: usize = 0;
    let final_size: usize;
    while ct <= nr_total {
        let f = format!(
            "{}/{}.part{:04}.split",
            dir.to_string_lossy(),
            file_name,
            ct
        );
        let p = Path::new(&f);
        println!("{:?}", p);

        //verific daca exista
        if !p.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Nu sunt prezente toate fisierele!!!",
            ));
        }
        let buffer = std::fs::read(p)?;
        if ct == nr_total {
            final_size = buffer.len();
            return Ok((size, final_size));
        } else {
            size = buffer.len();
        }
        ct += 1;
    }
    Err(io::Error::new(io::ErrorKind::Other, "Eroare la take_size"))
}

fn unsplit(file: &String) -> Result<(), io::Error> {
    //numele fisierului fara cale
    let file_name = take_file_from_path(file)?;

    //numele folderului corespunzator numelui fisierului
    let format_split = format!("{}_parts_splitted", file_name);
    let dir_parts_splitted = Path::new(&format_split);
    if !dir_parts_splitted.exists() {
        //daca exista incep sa fac unsplit
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Trebuie sa dai split inainte pentru a se forma folderul corespunzator cu fisiere split",
        ));
    }

    //creez fisierul de unsplit
    let format_unsplit = file.to_string();
    let mut file_unsplit = File::create(&format_unsplit).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Eroare la creare output file: {}", e),
        )
    })?;

    //variabila pentru nr parti
    let mut ct = 1;

    //aflam size-ul ditr-un fisier split
    let nr_total = take_number_split(dir_parts_splitted)?;
    let (size, final_size) = take_size(dir_parts_splitted, file_name, nr_total)?;

    //afisari partiale
    println!(
        "Dim fisierelor inafara de ultimul este {}, iar al ultimului {}",
        size, final_size
    );
    println!("{}", nr_total);

    while ct <= nr_total {
        let f = format!(
            "{}/{}.part{:04}.split",
            dir_parts_splitted.to_string_lossy(),
            file_name,
            ct
        );
        let p = Path::new(&f);

        //I verific daca exista
        if !p.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Nu sunt prezente toate fisierele!!!",
            ));
        }

        //II verific daca size-ul partilor = size
        let buffer_part = std::fs::read(p)?;

        println!("{}", buffer_part.len());

        if (buffer_part.len() != size && ct != nr_total)
            || (buffer_part.len() != final_size && ct == nr_total)
        {
            //verificam dimensiunea sa fie la fel
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Dimensiunea nu corespunde la : {:?}", p),
            ));
        }

        //concatenam partile in fisierlui de output

        file_unsplit.write_all(&buffer_part)?;

        ct += 1;
    }

    //III compar continutul daca e la fel

    //hash pentru unsplit
    let unsplit_p = Path::new(&format_unsplit);
    let buffer_unsplit = std::fs::read(unsplit_p)?;

    let mut hasher_unsplit = Sha256::new();
    hasher_unsplit.update(&buffer_unsplit);
    let result = hasher_unsplit.finalize();
    println!("{:?}", result);

    //comparam hash-ul original cu hash-ul unsplit

    //deschidem fisierul cu hash al fisierului original
    let buffer_hash_original = std::fs::read(format!("{}.hash", file_name))?;
    let original_hash: &[u8; 32] = &buffer_hash_original.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Nu corespunde la lungime la hash",
        )
    })?;

    println!("{:?}", original_hash);
    if result.as_slice() != original_hash {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Continutul nu este la fel. Fisiere corupte!!!",
        ));
    }
    println!("Ok");

    //sterg folderul de splitted si .hash

    fs::remove_dir_all(dir_parts_splitted)?;
    fs::remove_file(format!("{}.hash", file_name))?;

    Ok(())
}

fn main() {
    match Args::try_parse() {
        //se incearca parsarea argumentelor
        Ok(args) => {
            if args.help {
                println!("Comenzi disponibile:");
                println!("  split <file> -s <size>   Imparte <file> in mai multe parti.");
                println!("  unsplit <file>           Recombina fisierele .split");
                println!(
                    "Obs: <file> trebuie data toata calea daca nu se afla in folderul aplciatiei."
                );
                println!("     <size> poate fi 1b, 1kb, 1mb, _m, _, 1, etc.");
                return;
            }
            match args.command {
                Some(Commands::Split { file, size }) => {
                    let res = chunk(&size);
                    match res {
                        Ok(s) => match split(&file, s) {
                            Ok(()) => {}
                            Err(e) => {
                                println!("{:?}", e);
                            }
                        },
                        Err(e) => {
                            println!("{}", e);
                        }
                    }
                }
                Some(Commands::Unsplit { file }) => match unsplit(&file) {
                    Ok(()) => {}
                    Err(e) => {
                        println!("Eroare la unsplit {:?}", e)
                    }
                },
                None => {
                    println!("Comanda invalida!");
                }
            }
        }
        Err(_) => {
            println!("Scrie help pentru informatii despre cum trebuie intoduse argumentele");
        }
    }
}
