<h1>
  <img src="https://github.com/RonnChyran/seiri/blob/master/branding/seiri.png?raw=true" height="64" width="64">
  <ruby>
聲<rt>せい</rt>
理<rt>り</rt>
  </ruby>
  <em>seiri</em>
</h1>

🎶 Opinionated, barebones music manager.

## About
*seiri* is an opinionated music manager designed for large libraries (10000+ tracks). *Opinionated* means that *seiri* intentionally has one way of organizing music, and is not customizable besides very few options. This helps keeps large libraries consolidated and easily searachable. *seiri* doesn't care about the music player you use to listen to your library, or the tools used to tag your tracks, only the way the files are organized in folders. Because of this, *seiri* works best with music players that don't care where your music is stored, like [foobar2000](https://www.foobar2000.org/).

*seiri* is a backwards-compatible rewrite of [katatsuki](https://github.com/RonnChyran/Katatsuki) in Rust and React.


## What *seiri* does do.
* Move files around.
* Make your music queryable.
* Help keep large libraries organized.
* Does all these things fast.

## What *seiri* does not do.

*seiri* does not do any of the following. 

* [Play music.](https://www.foobar2000.org/)
* [Transcode music.](https://www.freac.org/)
* [Suggest music.](https://www.spotify.com/us/)
* [Identify music](https://www.shazam.com/)
* [Edit tags.](https://www.mp3tag.de/en/)
* [Lookup CDs.](https://picard.musicbrainz.org/)
* [Rip CDs](http://www.exactaudiocopy.de/).
* [Split CUE rips.](http://cue.tools/wiki/Main_Page)
* [Download music.](https://itunes.apple.com/ca/genre/music/id34)
* [Sync to your device.](https://getmusicbee.com/)
* [Upload music to a locker](https://play.google.com/music/)

## Rules

For *seiri* to work best, you should learn to accept its 6 easy rules.

1. Single-file CUE rips are **forbidden**
2. Your entire music library should be under one folder.
3. Tracks are sorted in an artist folder first, then album.
4. All music **must be properly tagged**. *seiri* won't accept "mymixtape.mp3" with no artist or album. This also means that WAV audio is **forbidden**. Use FLAC (or Wavpack if you really need that 32bit float).
5. Let *seiri* handle sorting for you. Do not move tracks around in the library folder. 
6. Cover art in the tags. **cover.jpg** is forbidden. Hard drive space is cheap. *seiri* will not care about your cover.jpg.

*seiri* works with most music formats, as long as you follow the rules above.

## Adding music
There is only one way to add music to your library with *seiri*. Next to your library folder, *seiri* will create an *Automatically add to Library* folder. Once you've finished tagging your music, move it to this folder, and *seiri* will move it to the proper place in your library folder, and index it in its database. 

You can delete or re-tag files in your library folder, but do not move it to another folder, or *seiri* will not be able to keep track of it. If you made a tag change, you can ask *seiri* to refresh it, and it will reorganize the track accordingly.

You can make top-level subfolders under the *Automatically add to Library* folder to keep track of the source. For example, if you had a *YouTube*\* folder, and an *iTunes*\* folder, *seiri* will automatically mark whether you got the track from iTunes, or YouTube, and make that queryable.

<sub>*If you need this, I hope you're not getting your music by ripping from YouTube 😉.</sub> 

## Queries
*seiri* supports querying your library using *bangs*. All bang inputs are case insensitive.

|Bang|Description|Inputs|
|----|-----------|------|
||Track Title Search|The empty bang matches all tracks in the database. In addition, a bang-less search matches track titles partially.|
|`!!`|The group bang|Another bang expression.|
|`!q`|Full Text Search|Matches track title, album title, artist partially.|
|`!Q`|Exact Full Text Search|Matches track title, album title, artist exactly.|
|`!al`|Album Title|Matches the name of the album partially.|
|`!AL`|Exact Album Title|Matches the name of the album exactly.|
|`!ala`|Album Artists|Matches the name of the album artist partially.|
|`!ALA`|Exact Album Artists|Matches the name of the album artist exactly.|
|`!f`|Format|`flac, mp3, alac, aac, vorbis, opus, wavpack` are self explanatory. The special tags `flac16, flac24` allow for distinction between FLAC bitrates, and `cbr, vbr` allow for distinction between constant bitrate MP3 and variable bitrate MP3.|
|`!br[lt\|gt]`|Bitrate strictly \[Less Than \| Greater Than\]|Integer|
|`!c(w\|h)[lt\|gt]`|Cover art has (width\|height) strictly \[Less Than \| Greater Than\]|Integer|
|`!c`|Has cover art in tags|`true` or `false`|
|`!mb`|Has [MusicBrainz](http://musicbrainz.org/) IDs in tags|`true` or `false`|
|`!dup`|Is a duplicate of another track (iTunes-like algorithm)|`true` or `false`|


Bangs can be combined with the logical symbols `&` (AND) and `|` (OR). The group bang `!!` is used to group multiple bangs together for scoping. There is also *true tick* syntax, where for bangs that take boolean values, can be written ``!dup` `` as shorthand for `!dup{true}`. If for some reason a closing brace `}` or backslash '\' occurs in your search, bangs support escape characters `\}` and `\\`.

Bangs are parsed and transpiled into SQLite statements, which are then executed on the library database for fast results.


## Building

*seiri* consists of multiple components.
 - *seiri-lib* is the main component written in Rust that handles database connections, monitoring of the library folder, and parsing and transpilation of query bangs. This library is automatically built as part of *seiri-watcher* and *seiri-client*.
 
 - *libkatatsuki*, is a forked version of Katatsuki's Track handling code, written in C#. We need this because the native version of [TagLib](http://taglib.org/) lacks features that [TagLibSharp](https://github.com/mono/taglib-sharp) implements that are required for compatible semantics with *Katatsuki*, and richer queries (such as cover-art size). Rust bindings are created using CoreRT to compile C# to native code, and a C interface.
 *libkatatsuki* and its Rust bindings are automatically built when building *seiri-watcher* and *seiri-client*.
 
 - *seiri-client* is an [Electron](https://github.com/electron/electron) application that handles interfacing with *seiri-client*, and acts as a watchdog in case *seiri-client* crashes, as well an automatic updater. We try to be mindful of memory usage, and usually start the Chrome render process only when needed. You will need to build this with `yarn build`.
 
 - *seiri-watcher* handles watching and adding new tracks. This should be built as part of *seiri-client*.
 
 - *seiri-neon* is the recommended way to interface with the core. It uses node's native extension support to call into Rust natively and interface with the Tracks database. This is built automatically with *seiri-client*.
 
 - *seiri-client-internals* is the actual user interface for *seiri-client*, consisting mostly of React code. This should be built as part of *seiri-client*.
 
 
 Read *build.md* for more information about setting up the environment.
