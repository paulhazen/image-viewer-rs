use std::env;

use crate::core::operations;
use crate::core::operations::OpType;
use crate::core::io;

use super::ppm::PpmImage;

// TODO: Should really have the result be a "read image" instead of just one
// that's in memory. There is a bunch of UI stuff that works differently if the
// image has been loaded directly from disk.... I think.
type ArgumentResult = Result<Option<PpmImage>, String>;

// Type for checking flags (index of argument, expected str, error message)
type FlagCheck = (&'static usize, &'static str, &'static str);

macro_rules! get_operation  {
    ($args_vec:ident) => {
      ($args_vec)[1].to_lowercase().as_str()
    };
}

/* #region Logic for parsing incoming arguments  */

/**
 * Parse the command-line arguments sent to the executable
 */
pub fn parse_arguments(arguments: Option<Vec<String>>) -> ArgumentResult {

  // TODO: https://doc.rust-lang.org/rust-by-example/flow_control/match.html

  let args = arguments.unwrap_or(env::args().collect());

  match args.len() {
    // this is here so that it will work if there are no arguments
    1 => { return Ok(None) },
    // can only be invert, or histeq
    6 => {
      match get_operation!(args) {
        "inv" => return parse_invert_command(&args),
        "histeq" => return parse_histeq_command(&args),
        "sobel" => return parse_sobel_command(&args),
        _ => return Err(
          format!("Unknown command: {}", get_operation!(args))
        ),
      }
    }
    // handles add, sub, mult, and histmatch
    7 => {
      match get_operation!(args) {
        "add" | "sub" | "mult" => return parse_image_operation_command(&args),
        "histmatch" => return parse_histmatch_command(&args),
        _ => return Err(
          format!("Unknown command: {}", get_operation!(args))
        ),
      }
    }
    // handles log, pow, and gblur
    10 => {
      match get_operation!(args) {
        "log" => return parse_log_command(&args),
        "pow" => return parse_pow_command(&args),
        "gblur" => return parse_gblur_command(&args),
        _ => return Err(
          format!("Unknown command {}", get_operation!(args))
        ),
      }
    }
    _ => return Err("Wrong number of arguments".to_string())
  }
}

/**
 * Parse any of the following image commands:
 * - Add
 * - Subtract
 * - Multiply
 */
fn parse_image_operation_command(args: &Vec<String>) -> ArgumentResult {

  const INPUT_FILE1:usize = 3;
  const INPUT_FILE2:usize = 4;

  let mut optype = OpType::Add;

  match args[1].to_lowercase().as_str() {
    "add" => optype = OpType::Add,
    "sub" => optype = OpType::Subtract,
    "mul" => optype = OpType::Multiply,
    _ => {}
  }

  // make sure input / output flags are in the right spots
  // TODO: Move this to the check_flag pattern
  if args[2].as_str() != "-i" {
    return Err("input flag is in the wrong place".to_string())
  } 
  if args[5].as_str() != "-o" {
    return Err("output flag is in the wrong place".to_string())
  }

  // load the two input images
  let lhs = io::open_image(
    args[INPUT_FILE1].as_str()
  ).unwrap();

  let rhs = io::open_image(
    args[INPUT_FILE2].as_str()
  ).unwrap();

  let op_result = operations::perform_operation(
    &lhs, &rhs, optype
  );

  match op_result {
    Ok(img) => {
      match io::write_image(&img, args[6].as_str()) {
        Err(why) => return Err(why.to_string()),
        Ok(_) => return Ok(Some(img)),
      }
    }
    Err(why) => {
      return Err(why)
    }
  }
}

/**
 * Parse the log command
 */
fn parse_log_command(args: &Vec<String>) -> ArgumentResult {
  
  const INPUT_FILE:usize = 7;
  const OUTPUT_FILE:usize = 9;

  let flag_checks: [FlagCheck; 4] = [
    (&2, "-c", "-c flag in the wrong place"),
    (&4, "-b", "-b flag in the wrong place"),
    (&6, "-i", "-i flag in the wrong place"),
    (&8, "-o", "-o flag in the wrong place"),
  ];

  let flag_check = do_flag_position_check(
    args, &flag_checks
  );
  
  match flag_check {
    Ok(_) => {
      // load the image
      // TODO: Remove the unchecked unwrap here - the open image operation
      // could still fail
      let mut input_image = io::open_image(
        args[INPUT_FILE].as_str()
      ).unwrap();

      // parse the c and b values
      let c: f32 = parse_float(&args[3]);
      let b: f32 = parse_float(&args[5]);

      // perform the log transform
      let log_result = operations::log_transform(
        &mut input_image, Some(c), Some(b)
      );

      // bleh... one of the annoying things about rust syntax I haven't been 
      // able to get around is all the nested matches... feels like there should
      // be a better way to do this...
      match log_result {
        Ok(mut image) => {
          match io::write_image(
            &mut image, args[OUTPUT_FILE].as_str()
          ) {
            Err(why) => return Err(why.to_string()),
            Ok(_) => return Ok(Some(image)),
          }
        },
        Err(why) => return Err(why.to_string()),
      }
    },
    Err(why) => Err(why.to_string()),
  }
}

/**
 * Parse the pow command
 */
fn parse_pow_command(args: &Vec<String>) -> ArgumentResult {

  const INPUT_FILE:usize = 7;
  const OUTPUT_FILE:usize = 9;

  use operations::gamma_transform;

  // array of tuples that contain the information for testing each flag, and
  // reporting an error if necessary.
  let flag_checks: [FlagCheck;4] = [
    (&2, "-c", "-c flag in the wrong place"),
    (&4, "-gamma", "-gamma flag in the wrong place"),
    (&6, "-i", "-i flag in the wrong place"),
    (&8, "-o", "-o flag in the wrong place"),
  ];

  let flag_check = do_flag_position_check(
    args,
    &flag_checks
  );

  match flag_check {
    Ok(_) => {
    // load the image
    let mut ppm = io::open_image(
      args[INPUT_FILE].as_str()
    ).unwrap();

    // parse the c and b values
    let c: f32 = parse_float(&args[3]);
    let gamma: f32 = parse_float(&args[5]);

    // perform the log transform
    let result = gamma_transform(
      &mut ppm, gamma,Some(c)
    );

    // if the result was a success
    match result {
      Ok(mut image) => {
        // write the file to the disk
        match io::write_image(
          &mut image, args[OUTPUT_FILE].as_str()
        ) {
          Err(why) => Err(why.to_string()),
          Ok(_) => Ok(Some(image)),
        }
      },
      Err(why) => Err(why.to_string()),
    }
  },
  Err(why) => Err(why.to_string()),
}
}

fn parse_gblur_command(_args: &Vec<String>) -> ArgumentResult {
  // TODO: Obviously this shit needs implementing
  Ok(None)
}

fn parse_sobel_command(_args: &Vec<String>) -> ArgumentResult {
  // TODO: Obviously this shit needs implementing
  Ok(None)
}
/**
 * Parse the invert command
 */
fn parse_invert_command(args: &Vec<String>) -> ArgumentResult {

  const INPUT_FILE:usize = 3;
  const OUTPUT_FILE:usize = 5;

  use crate::core::operations::negate;
  use crate::core::io::open_image;
  use crate::core::io::write_image;

  match args[1].to_lowercase().as_str() {
    "inv" => {

      let flag_check = do_flag_position_check(
        args, &[
        (&2, "-i", "-i flag in the wrong place"),
        (&4, "-o", "-o flag in the wrong place"),
      ]);
      
      match flag_check {
        Ok(_) => {
          let input_file = args[INPUT_FILE].as_str();
          let output_file = args[OUTPUT_FILE].as_str();

          // TODO: Handle the scenario where the image does not open
          if let Ok(mut input) = open_image(input_file){

            let negate_result = negate(
              &mut input
            );

            match negate_result {
              Ok(result) => {
                match write_image(&result, output_file) {
                  Ok(_) => return Ok(Some(result)),
                  Err(why) => return Err(why.to_string()),
                };
              },
              Err(why) => return Err(why.to_string()),
            }
        } else {
          Err("something went wrong opening the image file.".to_string())
        }
      },
      Err(why) => return Err(why.to_string()),
    }
  }
  _ => return Err("unknown command".to_string())
}
}

fn parse_histmatch_command(_args: &Vec<String>) -> ArgumentResult {
  // TODO: obviously this needs to be implemented....
  Ok(None)
}

fn parse_histeq_command(args: &Vec<String>) -> ArgumentResult {

  const INPUT_FILE:usize = 3;
  const OUTPUT_FILE:usize = 5;

  use crate::core::io::{open_image, write_image};
  use crate::core::operations::histogram_equalization;

  match args[1].to_lowercase().as_str() {
    "histeq" => {

      let flag_check = do_flag_position_check(
        args, &[
          (&2, "-i", "-i flag in the wrong place"),
          (&4, "-o", "-o flag in the wrong place"),
      ]);
      
      match flag_check {
        Ok(_) => {

          let input_file = args[INPUT_FILE].as_str();
          let output_file = args[OUTPUT_FILE].as_str();

          let open_result = open_image(
            input_file
          );

          match open_result {
            Ok(image) => {
              match histogram_equalization(&image, None) {
                Ok(eq_image) => {
                  match write_image(&eq_image, output_file) {
                    Ok(_) => Ok(Some(eq_image)),
                    Err(why) => Err(why.to_string()),
                  }
                },
                Err(why) => return Err(why.to_string())
              }            
            },
            Err(why) => return Err(why.to_string())
          }
        },
        Err(why) => return Err(why.to_string()),
      }
    },
    _ => return Err(
      format!("unknown command: {}", args[1].to_lowercase().as_str())
    ),
  }
}

fn parse_float(string: &String) -> f32 {
  // TODO: This is a little dangerous, because it silently returns zero if
  // it cannot parse the string into a float. Might want to look into doing this
  // better.
  string.parse::<>().unwrap_or(0.)
}

fn do_flag_position_check(
  args: &Vec<String>, 
  conditions: &[FlagCheck]) -> ArgumentResult {
  for flag_check in conditions {
    if args[*flag_check.0].to_lowercase().as_str() != flag_check.1 {
      return Err(flag_check.2.to_string());
    }
  }

  Ok(None)
}
/* #endregion */