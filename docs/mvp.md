# Project MVP

## Goal

Demonstrate datasets hosted in large public repositories can be:

- discovered
- structurally inspected
- scientifically labeled
- indexed
- searched by actual content

The MVP should establish a core architecture for crawling datasets, determining whether specific scientific content is actually present, and then making the classifications searchable. 

## MVP Scope

| Axis | MVP |
| --- | --- |
| Repository/source | Hugging Face + Zenodo |
| Artifact structure | Tabular + JSON |
| Scientific label | SMILES + Molecular Formula |
| Search | In-memory index + programmatic query/output |
| Validation | Utilizing existing SMILES and molecular-formula parsers |
| Test source | Bounded real datasets + deterministic local fixtures |

### MVP Feature Map

[x] ContentType
[x] Classification evidence
[x] Labeler concept
[x] Tabular visitor
[x] Visitor abstraction
[x] Formula labeler
[x] JSON visitor
[x] local end-to-end classification
[x] index
[x] crawler
[ ] logging support
[ ] Hugging Face
    [x] source/client implementation
    [x] bounded dataset search
    [x] search by scientific term
    [ ] optional Hub tag/filter support
    [ ] repository tree inspection
    [ ] identify supported artifacts (.csv / .json)
    [ ] reject oversized artifacts before download
    [ ] download supported artifact
    [ ] convert to CandidateArtifact
    [ ] classify + index through Crawler
    [ ] live end-to-end smoke test
    [ ] parquet tabular support
    [ ] plain text tabular support
[ ] Zenodo
[ ] evaluation

## Architecture

Separate project into inspection, crawling. Then later (not in MVP) serving.

`datasets-core -> datasets-crawler -> datasets-server (eventually)`

### `datasets-core`

Responsible for:

- visiting tabular and JSON data;
- identifying candidate fields;
- validating candidate values as SMILES or molecular formulae;
- returning classification results and supporting evidence.

Visitors need to be based on the data structure visited (tabular, json).

### `datasets-crawler`

Responsible for:

- retrieving candidate datasets from Hugging Face and Zenodo;
- identifying artifacts that can be inspected;
- passing artifacts to `datasets-core`;
- receiving classification results;
- contributing results to an index; and
- maintaining index for output

### Combined Functionality

The MPV should have the following functionality:

1. crawl or receive candidate datasets;
2. inspect supported tabular and JSON artifacts;
3. evaluate candidate fields for `SMILES` and `MolecularFormula`;
4. classify datasets based on those evaluations;
5. contribute classification results to an inverted index;
6. query the index by scientific content; and
7. output the matching datasets and their classification evidence.

### Milestones

1. Local Architecture
2. Real repository sources used
3. Evaluation of tool
