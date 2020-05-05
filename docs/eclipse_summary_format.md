# Eclipse Summary Format

Eclipse summary format consists of a pair of files: 

- a specification file (`.SMSPEC`)  describing the data layout;

- a "unified" summary file (`UNSMRY`) with the actual simulation output.

  ## Specification file

The specification is list of keywords with metadata describing how to interpret the data in the summary files. The `DIMENS` keyword in the specification file includes the parameter `NLIST` (first out of 5 integers in that keyword), which is the length of each data vector in the summary file. `STARTDAT` contains three integers that represent the simulation's start date as (year, month, day) tuple. Other useful bits of data for each data vector include:

- `KEYWORDS` -  an 8-char string for a vector name;

- `UNITS` - an 8-char string representing the physical units;

- `WGNAMES`/`NAMES` and `NUMS` - an fixed/dynamic length string and an integer that, together with a keyword name, identify the nature of a data vector (e.g. field data, well data, performance indicator);

  ## Summary file

In the summary file, data is recorded as report steps. Any report step can have one or more time steps, called ministeps. Report steps starts at 1, ministeps start at 0, but only ministeps are actually recorded in the file. Every report step starts with a `SEQHDR` keyword, followed by pairs of `MINISTEP`-`PARAMS` keywords. The `PARAMS` should be `NLIST` long.

A vector is uniquely identified by its own name and optionally the corresponding metadata values. For example:

- Time related vectors (no extra id): `TIME`,  `YEAR`; 

- Performance related vectors  (no extra id): `ELAPSED`,  `NEWTON`,  `NLINEARS`,  `TCPU`,  `TIMESTEP`;

- Vectors for field data (no extra id): `F...`,  `FMCT...` and `FMWPR/FMWIN`;

- Vectors that are associated with a well name from `WGNAMES`:
  - `W...`, e.g. `WOPR`, only need a well name;
  - `C...` also need a cell index from the `NUMS` keyword;
  
- Vectors that are associated with a group name from `WGNAMES`:
  
- `G...`, e.g. `GOPR`, only need a group name;
  
- Vectors for a cell or region numbers (`B...` and `R...`) need an additional index from `NUMS`;
