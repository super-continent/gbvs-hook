# gbvs-hook
BBScript extractor and loader for Granblue Fantasy: Versus

## How to Use This
Use the DLL Injector bundled with it by executing inject.bat (or use any DLL Injector of your preference) to load it inside GBVS, it should create a UI window that will allow you to extract scripts and set a mods folder. Currently this folder must be set on startup every time you inject the DLL.
To load these scripts, you must set your mod folder to the path containing all properly named .bbscript files. Main character scripts should be SHORTNAME.bbscript, ETC scripts should be SHORTNAME_ETC.bbscript, and CMNEF should be CMNEF.bbscript, the loader currently does not support stages or UNKNOWN 1-4 scripts.

### DISCLAIMER
Currently there are also a few issues with this program, the loader cannot currently keep track of which scripts correspond to what parts of the game accurately, so an internal counter seems to get misaligned if used on certain stages and certain mods, its recommended to use the default stage with training mode for consistency.
