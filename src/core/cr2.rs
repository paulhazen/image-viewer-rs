use std::{fs::File, io::{Seek, SeekFrom, Read, Cursor}};

use byteorder::{ByteOrder, ReadBytesExt};

use crate::core::ppm::PpmImage;

use super::io::IOResult;

/* #region Constants */
/// This is the byte order as defined in the CR2 spec that indicates little
/// endian-ness in the file
const LITTLE_ENDIAN:[u8;2] = [73, 73];

/// This is the byte order as defined in the CR2 spec that indicates big
/// endian-ness in the file
const BIG_ENDIAN:[u8;2] = [77, 77];

/// This is the flag that indicates the beginning of JPEG data
pub const START_OF_IMAGE:[u8;2] = [0xff, 0xd8];

/// This is the flag that indicates the end of JPEG data
pub const END_OF_IMAGE:[u8;2] = [0xff, 0xd9];

/// This marker indicates the start of the define huffman table header
pub const DEFINE_HUFFMAN_TABLE:[u8;2] = [0xff, 0xc4];

/// TIFF tag id for the height of an image
const IMAGE_HEIGHT:u16 = 48257;

/// TIFF tag id for the width of an image
const IMAGE_WIDTH:u16 = 48256;

/// TIFF tag id for the offset of image data within a TIFF file
const IMAGE_DATA_OFFSET:u16 = 273;

/* #endregion */

/* #region Data Structures */

/* #region CR2Image  */
#[derive(Clone)]
pub struct CR2Image {
  pub endianness:[u8;2],
  pub magic_number:u16,
  pub offset_to_first_ifd: u32,

  pub cr2_magic_word: String,
  pub cr2_major_version: u8,
  pub cr2_minor_version: u8,

  pub raw_ifd_offset:u32,

  pub images: Vec<ImageFileDirectory>,
}

impl CR2Image {
  pub fn new(
    endian: [u8;2], 
    magic_number: u16, 
    first_ifd_offset: u32,
    cr2_magic_word: String,
    cr2_major_version: u8,
    cr2_minor_version: u8,
    raw_ifd_offset:u32,
  ) -> Self {
    CR2Image { 
      endianness: endian, 
      magic_number: magic_number, 
      offset_to_first_ifd: first_ifd_offset, 
      cr2_magic_word: cr2_magic_word,
      cr2_major_version: cr2_major_version,
      cr2_minor_version: cr2_minor_version,
      raw_ifd_offset: raw_ifd_offset,
      images: Vec::new(), 
    }
  }
}

/* #endregion */

/* #region ImageFileDirectory */
#[derive(Clone)]
pub struct ImageFileDirectory {
  pub ifd_offset:u64,
  pub entries:Vec<IFDEntry>,
  pub data: ImageData,
}

impl ImageFileDirectory {
  pub fn new(offset: u64) -> Self {
    ImageFileDirectory { 
      ifd_offset: offset, 
      entries: Vec::new(),
      data: ImageData::new(),
    }
  }

  pub fn get_offset_to_image_data(&self) -> Option<u64> {
    let mut value: Option::<u64> = None;

    if let Some(data_offset) = self.get_entry_value(
      &IMAGE_DATA_OFFSET
    ) {
      value = Some(data_offset as u64)
    }

    value
  }

  pub fn get_image_height(&self) -> Option<u32> {
    self.get_entry_value(&IMAGE_HEIGHT)
  }

  pub fn get_image_width(&self) -> Option<u32> {
    self.get_entry_value(&IMAGE_WIDTH)
  }

  fn get_entry_value(&self, entry_id: &u16) -> Option<u32> {
    let mut value: Option::<u32> = None;

    for entry in &self.entries {
      if entry.tag_id == *entry_id {
        value = Some(entry.tag_value);
        break;
      }
    }

    value
  }
}
/* #endregion */

#[derive(Clone)]
pub struct DHTHeader {
  pub data:Vec<u8>
}

/* #region IFDEntry */
#[derive(Clone)]
pub struct IFDEntry {
  pub tag_id:u16,
  pub tag_type:u16,
  pub tag_count:u32,
  pub tag_value:u32, // could be a value or an offset to a value
  pub tag_string:String,
}

impl IFDEntry {
  pub fn new(tag_id: u16, tag_type: u16, tag_count: u32, tag_value: u32) -> Self {
    IFDEntry { 
      tag_id: tag_id, 
      tag_type: tag_type, 
      tag_count: tag_count, 
      tag_value: tag_value,
      tag_string: tag_value.to_string(),
    }
  }

  pub const fn entry_label(&self) -> &str {
    return get_tiff_label(self.tag_id);
  }

  pub const fn type_name(&self) -> &str {
    match self.tag_type {
      1 => { "ubyte, unsigned 8bits" },
      2 => { "string, ASCII, 0 terminated" },
      3 => { "ushort, unsigned 16 bits" },
      4 => { "ulong, unsigned 32 bits" },
      5 => { "urational, numerator & denominator ulongs" },
      6 => { "byte, signed 8 bits" },
      7 => { "ubyte sequence" },
      8 => { "short, signed 16 bits" },
      9 => { "long, signed 32 bits" },
      10 => { "rational, signed 2 longs" },
      11 => { "single precision (2 bytes) IEEE format" },
      12 => { "double precision (4 bytes) IEEE format" },
      _ => { "unknown type" }
    }
  }
}

/* #endregion */

/* #region ImageData */
#[derive(Clone)]
pub struct ImageData {
  pub data:Vec<u8>
}

impl ImageData {
  pub fn from_data(data: Vec<u8>) -> Self {
    ImageData {
      data: data
    }
  }

  pub fn new() -> Self {
    ImageData {
      data: Vec::new()
    }
  }

  pub fn parse_dht<T: ByteOrder>(&self) {
    let mut rdr = Cursor::new(&self.data);
    rdr.seek(SeekFrom::Start(2)); // skip the first two bytes

    let mut dht_end:u64 = 0;
    // check for the huffman marker
    if let Ok(huffman_marker) = rdr.read_u16::<T>() {
      if huffman_marker == 0xffc4 as u16 {
        dht_end = rdr.position();

        // get the size of the dht header
        if let Ok(dht_size) = rdr.read_u16::<T>() {
          dht_end += dht_size as u64;

          let table_class_index = rdr.read_u8().unwrap();

          println!("Table class /Huffman table index: {}", table_class_index);
        }

      }
    }
    // look for the define huffman table
    // the marker for the "Define Huffman Table" should be comprised of the 
    // third and fourth bytes
    if self.data[2..4] == DEFINE_HUFFMAN_TABLE {
      
      let mut rdr = Cursor::new(&self.data[4..6]);
      
      if let Ok(table_size) = rdr.read_u16::<T>() {
        println!("Length of Huffman Table: {}", table_size);

        

      }

    }
  }
}

/* #endregion */

/* #endregion */

/* #region Functions to Read CR2 Files */
pub fn read_cr2(path: &str) -> IOResult {

  println!("--- Reading \"{}\" ---", path);
  use byteorder::{BigEndian, LittleEndian};

  let temp = PpmImage::new(10, 10);

  if let Ok(mut file) = File::open(path) {

    /* #region Header */

    /* #region TIFF Header */
    let mut byte_order: [u8; 2] = [0; 2];
    if let Ok(_offset) = file.read_exact(&mut byte_order) {
      match byte_order {
        LITTLE_ENDIAN => {
          // the CR2 file in question has little endianness
          println!("Little endian!");
        },
        BIG_ENDIAN => {
          // the CR2 file in question has big endianness
          println!("Big endian!");
        },
        _ => { println!("Unknown Endian"); }
      }
    }

    // magic number is almost always 42
    let mut magic_number:u16 = 42;
    if byte_order == LITTLE_ENDIAN {
      magic_number = file.read_u16::<LittleEndian>().unwrap();
    } else {
      magic_number = file.read_u16::<BigEndian>().unwrap();
    }

    // offset to the first image file directory
    let mut offset_to_first_ifd: u32 = u32::default();
    if byte_order == LITTLE_ENDIAN {
      offset_to_first_ifd = file.read_u32::<LittleEndian>().unwrap();
    } else {
      offset_to_first_ifd = file.read_u32::<BigEndian>().unwrap();
    }
    /* #endregion */

    /* #region CR2 Header */

    // go to the eighth byte
    file.seek(SeekFrom::Start(8));

    // get the magic word
    let mut cr2_magic_word: String = "".to_string();
    let mut cr2_magic_word_byte_arr: [u8;2] = [0;2];
    if let Ok(_) = file.read_exact(&mut cr2_magic_word_byte_arr) {
      for c in cr2_magic_word_byte_arr {
        cr2_magic_word.push(c as char);
      }
    }

    // get the versions
    let cr2_major_version = file.read_u8().unwrap();
    let cr2_minor_version = file.read_u8().unwrap();

    println!("CR2 Version {}.{}", cr2_major_version, cr2_minor_version);
    // the offset to the start of the last IFD entry
    let mut raw_ifd_offset = u32::default();
    if byte_order == LITTLE_ENDIAN {
      raw_ifd_offset = file.read_u32::<LittleEndian>().unwrap();
    } else {
      raw_ifd_offset = file.read_u32::<BigEndian>().unwrap();
    }

    /* #endregion */

    println!("Done reading TIFF/CR2 file header");

    /* #endregion */

    let mut cr2_image = CR2Image::new(
      byte_order, 
      magic_number, 
      offset_to_first_ifd,
      cr2_magic_word,
      cr2_major_version,
      cr2_minor_version,
      raw_ifd_offset
    );

    /* #region Read the Image File Directory Entries */

    if byte_order == LITTLE_ENDIAN {
      read_all_ifd::<LittleEndian>(
        &mut file, &mut cr2_image, offset_to_first_ifd as u64
      );
    } else {
      read_all_ifd::<BigEndian>(
        &mut file, &mut cr2_image, offset_to_first_ifd as u64
      );
    }

    /* #endregion */

    println!("Finished parsing the CR2 file.");
  }

  Ok(temp)
}

fn read_all_ifd<T: ByteOrder>(
  file: &mut File, 
  cr2_image: &mut CR2Image,
  offset: u64
) {
  let mut index = 0;
  let mut current_offset = offset;
  while current_offset != 0 {
    println!("--- IFD#{} ----", index);
    let (mut ifd, new_offset) = parse_ifd::<T>(
      file, current_offset
    );
    
    /*
    
    if let Some(data_offset) = ifd.get_offset_to_image_data() {
      ifd.data = read_image_data::<T>(
        file, data_offset
      );


      let file = format!("Temp{}.jpg", index);
      let path = std::path::Path::new(file.as_str());
      let display = path.display();
      let file = match File::create(&path) {
        Err(why) => panic!("Couldn't create {}: {}", display, why),
        Ok(file) => file,
      };
    
      let mut file_buffer = BufWriter::new(file);
    
      match file_buffer.write_all(&ifd.data.data){
        Err(why) => { panic!("Couldn't write to file buffer: {}", why)},
        Ok(_) => {},
      };
    } else {
      ifd.data.data = Vec::new();
    }

    */

    current_offset = new_offset as u64;
    cr2_image.images.push(ifd);
    index += 1;
  }
}

fn read_image_data<T: ByteOrder>(
  file: &mut File, start_marker: u64
) -> ImageData {
  let mut image_data = ImageData::new();

  // store the current stream position
  let old_stream_position = file.stream_position().unwrap();

  if let Ok(_) = file.seek(SeekFrom::Start(start_marker)) {
    while let Ok(byte) = file.read_u8() {
      
      // add byte to the image data array
      image_data.data.push(byte);

      // check for end of image flag
      if image_data.data.len() > 1 {
        let last_two = [
          image_data.data[image_data.data.len() - 2],
          image_data.data[image_data.data.len() - 1]
        ];

        if last_two == END_OF_IMAGE {
          break;
        }
      }
    }
  }

  // return to the last position that the file stream was at
  file.seek(SeekFrom::Start(old_stream_position));

  image_data.parse_dht::<T>();

  image_data
}

fn parse_ifd<T: ByteOrder>(file: &mut File, offset: u64) -> (ImageFileDirectory, u32) {
  let mut ifd = ImageFileDirectory::new(offset);

  // go to the offset for the image file directory
  if let Ok(_) = file.seek(SeekFrom::Start(offset)) {
    // get the number of entries
    let entry_count = file.read_u16::<T>().unwrap();

    println!("Entries in IFD: {}", entry_count);

    // set the capacity of the vector to minimize memory allocations
    ifd.entries = Vec::with_capacity(entry_count as usize);

    for _ in 0..entry_count {
      let mut entry = IFDEntry::new(
        file.read_u16::<T>().unwrap(),
        file.read_u16::<T>().unwrap(),
        file.read_u32::<T>().unwrap(),
        file.read_u32::<T>().unwrap()
      );

      let ifd_position = file.stream_position().unwrap();
      // if the tag type is 2, then it's an ASCII value
      if entry.tag_type == 2 {
        // seek to the place in the file that contains the value
        if let Ok(_) = file.seek(SeekFrom::Start(entry.tag_value as u64)) {
          let mut string_bytes:Vec<u8> = vec![0;entry.tag_count as usize];//Vec::with_capacity(entry.tag_count as usize);
          if let Ok(_) = file.read_exact(&mut string_bytes) {
            if let Ok(string_value) = std::str::from_utf8(&string_bytes) {
              entry.tag_string = string_value.to_string();
            }
          }
        }
      }

      // seek back to the position in the IFD
      if let Err(_) = file.seek(SeekFrom::Start(ifd_position)) {
        break;
      }

      println!(
        "{}: {} ({}) LEN: {}", 
        entry.entry_label(), 
        entry.tag_string, 
        entry.type_name(),
        entry.tag_count
      );

      ifd.entries.push(entry);
    }
  }

  let next_ifd_offset = file.read_u32::<T>().unwrap();

  (ifd, next_ifd_offset)
}

/* #endregion */

/* #region TIFF Label stuff */

const fn get_tiff_label(number:u16) -> &'static str {
  match number {
    254 => "NewSubfileType",
    255 => "SubfileType",
    256 => "ImageWidth",
    257 => "ImageLength",
    258 => "BitsPerSample",
    259 => "Compression",
    262 => "PhotometricInterpretation",
    263 => "Threshholding",
    264 => "CellWidth",
    265 => "CellLength",
    266 => "FillOrder",
    269 => "DocumentName",
    270 => "ImageDescription",
    271 => "Make",
    272 => "Model",
    273 => "StripOffsets", // this is an important one
    274 => "Orientation",
    277 => "SamplesPerPixel",
    278 => "RowsPerStrip",
    279 => "StripByteCounts",
    280 => "MinSampleValue",
    281 => "MaxSampleValue",
    282 => "XResolution",
    283 => "YResolution",
    284 => "PlanarConfiguration",
    285 => "PageName",
    286 => "XPosition",
    287 => "YPosition",
    288 => "FreeOffsets",
    289 => "FreeByteCounts",
    290 => "GrayResponseUnit",
    291 => "GrayResponseCurve",
    292 => "T4Options",
    293 => "T6Options",
    296 => "ResolutionUnit",
    297 => "PageNumber",
    301 => "TransferFunction",
    305 => "Software",
    306 => "DateTime",
    315 => "Artist",
    316 => "HostComputer",
    317 => "Predictor",
    318 => "WhitePoint",
    319 => "PrimaryChromaticities",
    320 => "ColorMap",
    321 => "HalftoneHints",
    322 => "TileWidth",
    323 => "TileLength",
    324 => "TileOffsets",
    325 => "TileByteCounts",
    326 => "BadFaxLines",
    327 => "CleanFaxData",
    328 => "ConsecutiveBadFaxLines",
    330 => "SubIFDs",
    332 => "InkSet",
    333 => "InkNames",
    334 => "NumberOfInks",
    336 => "DotRange",
    337 => "TargetPrinter",
    338 => "ExtraSamples",
    339 => "SampleFormat",
    340 => "SMinSampleValue",
    341 => "SMaxSampleValue",
    342 => "TransferRange",
    343 => "ClipPath",
    344 => "XClipPathUnits",
    345 => "YClipPathUnits",
    346 => "Indexed",
    347 => "JPEGTables",
    351 => "OPIProxy",
    400 => "GlobalParametersIFD",
    401 => "ProfileType",
    402 => "FaxProfile",
    403 => "CodingMethods",
    404 => "VersionYear",
    405 => "ModeNumber",
    433 => "Decode",
    434 => "DefaultImageColor",
    512 => "JPEGProc",
    513 => "ThumbnailOffset",
    514 => "ThumbnailLength",
    515 => "JPEGRestartInterval",
    517 => "JPEGLosslessPredictors",
    518 => "JPEGPointTransforms",
    519 => "JPEGQTables",
    520 => "JPEGDCTables",
    521 => "JPEGACTables",
    529 => "YCbCrCoefficients",
    530 => "YCbCrSubSampling",
    531 => "YCbCrPositioning",
    532 => "ReferenceBlackWhite",
    559 => "StripRowCounts",
    700 => "XMP",
    18246 => "Image.Rating",
    18249 => "Image.RatingPercent",
    32781 => "ImageID",
    32932 => "Wang Annotation",
    33421 => "CFARepeatPatternDim",
    33422 => "CFAPattern",
    33423 => "BatteryLevel",
    33432 => "Copyright",
    33434 => "ExposureTime",
    33437 => "FNumber",
    33445 => "MD FileTag",
    33446 => "MD ScalePixel",
    33447 => "MD ColorTable",
    33448 => "MD LabName",
    33449 => "MD SampleInfo",
    33450 => "MD PrepDate",
    33451 => "MD PrepTime",
    33452 => "MD FileUnits",
    33550 => "ModelPixelScaleTag",
    33723 => "IPTC/NAA",
    33918 => "INGR Packet Data Tag",
    33919 => "INGR Flag Registers",
    33920 => "IrasB Transformation Matrix",
    33922 => "ModelTiepointTag",
    34016 => "Site",
    34017 => "ColorSequence",
    34018 => "IT8Header",
    34019 => "RasterPadding",
    34020 => "BitsPerRunLength",
    34021 => "BitsPerExtendedRunLength",
    34022 => "ColorTable",
    34023 => "ImageColorIndicator",
    34024 => "BackgroundColorIndicator",
    34025 => "ImageColorValue",
    34026 => "BackgroundColorValue",
    34027 => "PixelIntensityRange",
    34028 => "TransparencyIndicator",
    34029 => "ColorCharacterization",
    34030 => "HCUsage",
    34031 => "TrapIndicator",
    34032 => "CMYKEquivalent",
    34033 => "Reserved",
    34034 => "Reserved",
    34035 => "Reserved",
    34264 => "ModelTransformationTag",
    34377 => "Photoshop",
    34665 => "Exif IFD",
    34675 => "InterColorProfile",
    34732 => "ImageLayer",
    34735 => "GeoKeyDirectoryTag",
    34736 => "GeoDoubleParamsTag",
    34737 => "GeoAsciiParamsTag",
    34850 => "ExposureProgram",
    34852 => "SpectralSensitivity",
    34853 => "GPSInfo",
    34855 => "ISOSpeedRatings",
    34856 => "OECF",
    34857 => "Interlace",
    34858 => "TimeZoneOffset",
    34859 => "SelfTimeMode",
    34864 => "SensitivityType",
    34865 => "StandardOutputSensitivity",
    34866 => "RecommendedExposureIndex",
    34867 => "ISOSpeed",
    34868 => "ISOSpeedLatitudeyyy",
    34869 => "ISOSpeedLatitudezzz",
    34908 => "HylaFAX FaxRecvParams",
    34909 => "HylaFAX FaxSubAddress",
    34910 => "HylaFAX FaxRecvTime",
    36864 => "ExifVersion",
    36867 => "DateTimeOriginal",
    36868 => "DateTimeDigitized",
    37121 => "ComponentsConfiguration",
    37122 => "CompressedBitsPerPixel",
    37377 => "ShutterSpeedValue",
    37378 => "ApertureValue",
    37379 => "BrightnessValue",
    37380 => "ExposureBiasValue",
    37381 => "MaxApertureValue",
    37382 => "SubjectDistance",
    37383 => "MeteringMode",
    37384 => "LightSource",
    37385 => "Flash",
    37386 => "FocalLength",
    37387 => "FlashEnergy",
    37388 => "SpatialFrequencyResponse",
    37389 => "Noise",
    37390 => "FocalPlaneXResolution",
    37391 => "FocalPlaneYResolution",
    37392 => "FocalPlaneResolutionUnit",
    37393 => "ImageNumber",
    37394 => "SecurityClassification",
    37395 => "ImageHistory",
    37396 => "SubjectLocation",
    37397 => "ExposureIndex",
    37398 => "TIFF/EPStandardID",
    37399 => "SensingMethod",
    37500 => "MakerNote",
    37510 => "UserComment",
    37520 => "SubsecTime",
    37521 => "SubsecTimeOriginal",
    37522 => "SubsecTimeDigitized",
    37724 => "ImageSourceData",
    40091 => "XPTitle",
    40092 => "XPComment",
    40093 => "XPAuthor",
    40094 => "XPKeywords",
    40095 => "XPSubject",
    40960 => "FlashpixVersion",
    40961 => "ColorSpace",
    40962 => "PixelXDimension",
    40963 => "PixelYDimension",
    40964 => "RelatedSoundFile",
    40965 => "Interoperability IFD",
    41483 => "FlashEnergy",
    41484 => "SpatialFrequencyResponse",
    41486 => "FocalPlaneXResolution",
    41487 => "FocalPlaneYResolution",
    41488 => "FocalPlaneResolutionUnit",
    41492 => "SubjectLocation",
    41493 => "ExposureIndex",
    41495 => "SensingMethod",
    41728 => "FileSource",
    41729 => "SceneType",
    41730 => "CFAPattern",
    41985 => "CustomRendered",
    41986 => "ExposureMode",
    41987 => "WhiteBalance",
    41988 => "DigitalZoomRatio",
    41989 => "FocalLengthIn35mmFilm",
    41990 => "SceneCaptureType",
    41991 => "GainControl",
    41992 => "Contrast",
    41993 => "Saturation",
    41994 => "Sharpness",
    41995 => "DeviceSettingDescription",
    41996 => "SubjectDistanceRange",
    42016 => "ImageUniqueID",
    42032 => "CameraOwnerName",
    42033 => "BodySerialNumber",
    42034 => "LensSpecification",
    42035 => "LensMake",
    42036 => "LensModel",
    42037 => "LensSerialNumber",
    42112 => "GDAL_METADATA",
    42113 => "GDAL_NODATA",
    48129 => "PixelFormat",
    48130 => "Transformation",
    48131 => "Uncompressed",
    48132 => "ImageType",
    48256 => "ImageWidth",
    48257 => "ImageHeight",
    48258 => "WidthResolution",
    48259 => "HeightResolution",
    48320 => "ImageOffset",
    48321 => "ImageByteCount",
    48322 => "AlphaOffset",
    48323 => "AlphaByteCount",
    48324 => "ImageDataDiscard",
    48325 => "AlphaDataDiscard",
    48132 => "ImageType",
    50215 => "Oce Scanjob Description",
    50216 => "Oce Application Selector",
    50217 => "Oce Identification Number",
    50218 => "Oce ImageLogic Characteristics",
    50341 => "PrintImageMatching",
    50706 => "DNGVersion",
    50707 => "DNGBackwardVersion",
    50708 => "UniqueCameraModel",
    50709 => "LocalizedCameraModel",
    50710 => "CFAPlaneColor",
    50711 => "CFALayout",
    50712 => "LinearizationTable",
    50713 => "BlackLevelRepeatDim",
    50714 => "BlackLevel",
    50715 => "BlackLevelDeltaH",
    50716 => "BlackLevelDeltaV",
    50717 => "WhiteLevel",
    50718 => "DefaultScale",
    50719 => "DefaultCropOrigin",
    50720 => "DefaultCropSize",
    50721 => "ColorMatrix1",
    50722 => "ColorMatrix2",
    50723 => "CameraCalibration1",
    50724 => "CameraCalibration2",
    50725 => "ReductionMatrix1",
    50726 => "ReductionMatrix2",
    50727 => "AnalogBalance",
    50728 => "AsShotNeutral",
    50729 => "AsShotWhiteXY",
    50730 => "BaselineExposure",
    50731 => "BaselineNoise",
    50732 => "BaselineSharpness",
    50733 => "BayerGreenSplit",
    50734 => "LinearResponseLimit",
    50735 => "CameraSerialNumber",
    50736 => "LensInfo",
    50737 => "ChromaBlurRadius",
    50738 => "AntiAliasStrength",
    50739 => "ShadowScale",
    50740 => "DNGPrivateData",
    50741 => "MakerNoteSafety",
    50752 => "CR2Slices",
    50778 => "CalibrationIlluminant1",
    50779 => "CalibrationIlluminant2",
    50780 => "BestQualityScale",
    50781 => "RawDataUniqueID",
    50784 => "Alias Layer Metadata",
    50827 => "OriginalRawFileName",
    50828 => "OriginalRawFileData",
    50829 => "ActiveArea",
    50830 => "MaskedAreas",
    50831 => "AsShotICCProfile",
    50832 => "AsShotPreProfileMatrix",
    50833 => "CurrentICCProfile",
    50834 => "CurrentPreProfileMatrix",
    50879 => "ColorimetricReference",
    50931 => "CameraCalibrationSignature",
    50932 => "ProfileCalibrationSignature",
    50933 => "ExtraCameraProfiles",
    50934 => "AsShotProfileName",
    50935 => "NoiseReductionApplied",
    50936 => "ProfileName",
    50937 => "ProfileHueSatMapDims",
    50938 => "ProfileHueSatMapData1",
    50939 => "ProfileHueSatMapData2",
    50940 => "ProfileToneCurve",
    50941 => "ProfileEmbedPolicy",
    50942 => "ProfileCopyright",
    50964 => "ForwardMatrix1",
    50965 => "ForwardMatrix2",
    50966 => "PreviewApplicationName",
    50967 => "PreviewApplicationVersion",
    50968 => "PreviewSettingsName",
    50969 => "PreviewSettingsDigest",
    50970 => "PreviewColorSpace",
    50971 => "PreviewDateTime",
    50972 => "RawImageDigest",
    50973 => "OriginalRawFileDigest",
    50974 => "SubTileBlockSize",
    50975 => "RowInterleaveFactor",
    50981 => "ProfileLookTableDims",
    50982 => "ProfileLookTableData",
    51008 => "OpcodeList1",
    51009 => "OpcodeList2",
    51022 => "OpcodeList3",
    51041 => "NoiseProfile",
    51089 => "OriginalDefaultFinalSize",
    51090 => "OriginalBestQualityFinalSize",
    51091 => "OriginalDefaultCropSize",
    51107 => "ProfileHueSatMapEncoding",
    51108 => "ProfileLookTableEncoding",
    51109 => "BaselineExposureOffset",
    51110 => "DefaultBlackRender",
    51111 => "NewRawImageDigest",
    51112 => "RawToPreviewGain",
    51125 => "DefaultUserCrop",
    _ => "Unknown",
  }
}

/* #endregion */

