use std::fs;
use std::io::Write;

use crate::{
    args::Args, coordinate_system::transformation::CoordTransformer, data_processing, ground,
    map_transformation, osm_parser, retrieve_data,
};

pub fn execute_generator(args: Args) {
    let mut ground = ground::generate_ground_data(&args);

    let (coord_transformer, outer_box) =
        CoordTransformer::llbbox_to_xzbbox(&args.bbox, args.scale).unwrap();

    let batches = match args.batch_area_size {
        Some(x) => outer_box.batch(x),
        None => vec![outer_box],
    };

    for xzbbox in batches {
        // Fetch data
        let raw_data = match &args.file {
            Some(file) => retrieve_data::fetch_data_from_file(file),
            None => retrieve_data::fetch_data_from_overpass(
                args.bbox,
                args.debug,
                args.downloader.as_str(),
                args.save_json_file.as_deref(),
            ),
        }
        .expect("Failed to fetch data");

        // Parse raw data
        let (mut parsed_elements, mut xzbbox) =
            osm_parser::parse_osm_data(raw_data, xzbbox, &coord_transformer, args.debug);
        parsed_elements.sort_by_key(|element: &osm_parser::ProcessedElement| {
            osm_parser::get_priority(element)
        });

        // Write the parsed OSM data to a file for inspection
        if args.debug {
            let mut buf = std::io::BufWriter::new(
                fs::File::create("parsed_osm_data.txt").expect("Failed to create output file"),
            );
            for element in &parsed_elements {
                writeln!(
                    buf,
                    "Element ID: {}, Type: {}, Tags: {:?}",
                    element.id(),
                    element.kind(),
                    element.tags(),
                )
                .expect("Failed to write to output file");
            }
        }

        // Transform map (parsed_elements). Operations are defined in a json file
        map_transformation::transform_map(&mut parsed_elements, &mut xzbbox, &mut ground);

        // Generate world
        let _ = data_processing::generate_world(&parsed_elements, xzbbox, &ground, &args);
    }
}
