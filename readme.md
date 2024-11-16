# directory-archiver

`directory-archiver` is a Rust-based command-line application that can be used to archive many directories to a single file based on different profiles.

## Overview

The general idea of this application is that it can be used to back-up a configurable collection of directories dictated by individual profiles to a single, compressed ZIP file. This is primarily used to facilitate the easy preservation of a fixed set of directories; of which the structure of the resulting archive contains each of the directories at the common interesection of their parent directories.

## Usage

Archive using the profile at the specified path, creating the specified output file:

```
directory-archiver --profile documents.json --file output.zip
```

## Profiles

Profiles are represented by individual JSON files, which contain the name of the profile, the directories to be preserved whenever the profile is used, and the names of any paths to ignore. For example, to create a profile named 'Simple' that archives a single directory and ignores anything with a specific name, it would appear as follows:

```json
{
	"name": "Simple",
	"dirs": [
		"C:\\Documents"
	],
	"ignores": [
		"desktop.ini"
	]
}
```
