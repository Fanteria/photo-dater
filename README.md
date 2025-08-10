# Photo Dater

[![codecov](https://codecov.io/gh/Fanteria/photo-dater/graph/badge.svg?token=L3VUO5ETEZ)](https://codecov.io/gh/Fanteria/photo-dater)

A command-line tool for organizing photo directories and files based on EXIF creation dates. Photo Dater helps you maintain consistent naming conventions and organization schemes for your photo collections.

## Features

- **Directory Name Validation**: Check if directory names match the date range of contained photos
- **Smart Directory Renaming**: Automatically rename directories based on photo creation dates  
- **File Organization**: Move photos into date-based subdirectories
- **Sequential File Renaming**: Rename files with consistent numbering schemes
- **EXIF Date Extraction**: Reads creation dates from photo metadata
- **Flexible Date Range Support**: Handles single dates and date ranges with intelligent formatting

## Installation

### From Source

```bash
git clone <repository-url>
cd photo-dater
cargo build --release
```

The binary will be available at `target/release/photo-dater`.

### Prerequisites

- Rust 1.70+ (or whatever your MSRV is)
- Photos with EXIF metadata containing `DateTimeOriginal` field

## Usage 

For complete usage information, run `photo-dater --help`.

**Basic syntax**: `photo-dater [DIRECTORY] <COMMAND>`

If no directory is specified, the current directory (`.`) is used.

### Examples

```bash
# Check directory naming status
photo-dater ./vacation-photos/ status

# Rename directory if all photos are from the same day
photo-dater ./vacation-photos/ rename

# Allow up to 7 days between oldest and newest photos
photo-dater ./week-trip/ rename --max-interval 7 

# Preview what would be renamed
photo-dater ./my-photos/ rename --dry-run

# Display all files sorted by creation date
photo-dater ./my-photos/ list

# Display the date interval of files
photo-dater ./my-photos/ interval

# Check if all photos span no more than 3 days
photo-dater ./my-photos/ check 3

# Rename files using directory name as base
photo-dater ./vacation/ files-rename 

# Use custom base name
photo-dater ./vacation/ files-rename --name "beach-trip"

# Move files into date-based subdirectories
photo-dater ./my-photos/ move-by-days --dry-run
```

### Organizing Files by Date

The `move-by-days` command creates subdirectories named by date (YYYY-MM-DD) and moves photos based on their creation date:

```bash
# Before:
./vacation/
├── IMG_001.jpg (created 2025-05-01)
├── IMG_002.jpg (created 2025-05-01) 
└── IMG_003.jpg (created 2025-05-03)

# After running: photo-dater ./vacation/ move-by-days
./vacation/
├── 2025-05-01/
│   ├── IMG_001.jpg
│   └── IMG_002.jpg
└── 2025-05-03/
    └── IMG_003.jpg
```

## Directory Naming Conventions

Photo Dater recognizes and generates directory names in these formats:

### Single Date
- `2025-05-01 My Photos` - All photos from May 1st, 2025

### Date Ranges
- `2025-05-01 - 2025-05-03 My Photos` - Full date range
- `2025-05-01 - 05-03 My Photos` - Same year, different months
- `2025-05-01 - 03 My Photos` - Same month, different days

## Supported File Formats

Photo Dater works with any image file containing EXIF metadata with a `DateTimeOriginal` field:

- **JPEG** (.jpg, .jpeg)
- **TIFF** (.tiff, .tif) and some RAW formats based on TIFF
- **HEIF/HEIC/AVIF** - Modern formats from Apple and others
- **PNG** - When EXIF data is present
- **WebP** - When EXIF data is present

Files without EXIF data or creation dates are automatically skipped.

For more information about supported formats, see the [exif-rs documentation](https://github.com/kamadak/exif-rs?tab=readme-ov-file).
