# marblecomic
A simple comic reader with a reading tracker, using rocket and Maud

It host a website, that can be accessed on the internal netword (depending your network configuration)

I made it for reading my mirror of canterlotcomics.com . I'll share the mirroring tool later, once I've finished mirroring this website.

The comic specification ares:
- each comic is represent by one folder containing :

- a data.json that contain the following root entry:
- id: a unique id for this comic. It should be a small number, and no other comic should have this id
- comic_name: an optional string, that contain the comic name
- description: an optional string, that contain the comic description
- keywords: A dictionary with string as key (keyword category) and list of string as value (keyword this comic correspond to in the keyword category)
- translations: a list of pair ( like ["en", 1] ) with each pair having for first value a string with the language name (use the same consistently) and the comic id of the translation.
- found: should be true. If not, the comic is considered as if it doesn't exist.

for each picture of the comic, it need to be in the folder, under the form xxxxx-yyyyy.ext where :
- xxxxx is an unsigned integer. This is the chapter number.
- yyyyy is an unsigned integer. This is the page number inside the chapter.
- ext is a string. That's just the file extension.

You need to change the path to the folder having all the comic as subfolder in main.rs, at the line that contain load_from_dir

You can also set enable_progress_writing to false if you don't want to allow writing the reading progress (like if you wan't that multiple user use it). It will still read and display progress, however.
