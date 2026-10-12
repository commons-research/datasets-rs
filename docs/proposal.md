# Proposal: `metabolomics-datasets-rs`: Programmatic Interoperability for Heterogeneous Metabolomics Datasets

## Question

What shared dataset-level properties are required to represent and materialize heterogenous metabolomics datasets through a common programmatic interface, while preserving the information for downstream access? And how much redesign is necessary between datasets? 

## Insights and Motivation
Public metabolomics and related chemical datasets are distributed through heterogeneous repositories, APIs, archives, file layouts, versions, metadata models, and licensing schemes. Individual resources may be well structured internally, but downstream software often requires dataset-specific knowledge simply to discover, acquire, identify, and prepare a dataset for use. This created duplicated acquisition logic across applications and makes it difficult to treat datasets as interchangeable scientific resources. 

This project will investigate whether a shared dataset-level model can describe sufficiently heterogenous resources without becoming either overly dataset-specific or generic enough that little interoperability beyond a convenience store of URLs is available. The implementation, `metabolomics-datasets-rs`, will therefore serve both as a reusable software artifact and as an experimental instrument: adding each new dataset tests whether the shared representation remains stable, which source-specific adaptations are necessary, and where generalization fails. 

The project is expected to produce three principal scientific insights:

1. Characterization of current dataset-level interoperability: Which properties of publicly available metabolomics datasets are already sufficiently consisten to support shared programmatic treatment, and which remain specific to a repository or dataset? 
2. Tested set of requirements for reusable dataset representation: Possible properties include stable identity, acquisition source, content, citation and licensing information, version/provenance information.
3. Empirical account of generalization limitations: Cases where a new dataset requires modification of the shared model, has difficulty being materialized in a reproducible manner, lacks metadata deemed necessary, or requires overly tailored structure will all be documented. 

This project directly supports the data-modelling and validation axis of my broader PhD project. The doctoral research asks which shared structural constraints are sufficient to map heterogeneous experimental metabolomics datasets to reusable data models, without redesigning the model for each dataset. This project investigates an earlier dataset-level layer of that same problem: Datasets must first be identifiable, obtainable, described, versioned (when available), all before these heterogeneous experimental metabolomic datasets can be structurally or semantically integrated downstream. Ideally this process should be reproducible and reliable, while the metadata and provenance requirements may potentially inform later semantic-interoperability work.

### Impact on Commons Lab

**Downstream Applicatoin:** A common dataset acquisition layer would remove dataset-specific logic for fetching and working with datasets in lab crates doing analysis and parsing. Dataset resources such as LOTUS and PubChem could be obtained through a common interface, for use in downstream lab software. Moreover datasets could be downloaded and used based on specifications of the dataset content, such retrieving all datasets with a SMILES representation, allowing for more rapid data analysis and prototyping. 

**Dataset Creation and Publication**: Through attempting to represent these datasets through one model should expose practical requirements for publishing reusable scientific datasets. These findings can inform how Commons Lab resources are released, including which identity, versioning, licensing, citation, provenance, and artifact information should be provided for reproducible downstream use. 

## Relevant Work

| Work | Main finding | Relevance to this project | Remaining gap |
|---|---|---|---|
| Wilkinson et al., **“The FAIR Guiding Principles for scientific data management and stewardship,”** *Scientific Data*, 2016. [Nature](https://www.nature.com/articles/sdata201618) | Defines the FAIR principles of findability, accessibility, interoperability, and reusability, with an emphasis on machine-actionable scientific data and metadata. | Provides the conceptual foundation for evaluating whether a shared representation and access layer improves the usability and interoperability of metabolomics datasets. | FAIR defines desired properties of interoperable and reusable scientific data but intentionally leaves domain-specific implementation and technical mechanisms unspecified. |
| van der Werf et al., **“Standard reporting requirements for biological samples in metabolomics experiments: microbial and in vitro biology experiments,”** *Metabolomics*, 2007. [Springer](https://link.springer.com/article/10.1007/s11306-007-0080-4) | Establishes metabolomics reporting requirements intended to improve evaluation, comparison, and reinvestigation of experimental results. | Demonstrates that interoperability problems in metabolomics have long been recognized and that consistent representation of metadata is necessary for comparison and reuse. | Establishes reporting requirements for experiments but does not address programmatic acquisition or normalization of independently distributed datasets. |
| Hoffmann et al., **“mzTab-M: A Data Standard for Sharing Quantitative Results in Mass Spectrometry Metabolomics,”** *Analytical Chemistry*, 2019. [PubMed](https://pubmed.ncbi.nlm.nih.gov/30688441/) | Introduces a standardized format for exchanging quantitative mass-spectrometry metabolomics results using structured fields and controlled vocabulary. | Provides a concrete example of improving interoperability by imposing a common representation over otherwise heterogeneous metabolomics outputs. | Standardizes a particular class of metabolomics results but does not provide discovery, acquisition, or a common access abstraction across heterogeneous external datasets and repositories. |
| Haug et al., **“MetaboLights: a resource evolving in response to the needs of its scientific community,”** *Nucleic Acids Research*, 2020. [OUP Academic](https://academic.oup.com/nar/article/48/D1/D440/5613675) | Describes a mature public metabolomics repository designed around metabolomics data, associated metadata, and community requirements. | Demonstrates the benefits of structured data and metadata within a domain-specific repository and provides an important class of resource for this project. | Provides interoperability within the MetaboLights ecosystem but not a common programmatic abstraction across independently maintained resources. |
| Sud et al., **“Metabolomics Workbench: An international repository for metabolomics data and metadata, metabolite standards, protocols, tutorials and training, and analysis tools,”** *Nucleic Acids Research*, 2016. [OUP Academic](https://academic.oup.com/nar/article/44/D1/D463/2502588) | Presents a public repository combining metabolomics datasets with metadata, standards, protocols, and analysis resources. | Provides another major example of repository-level organization and illustrates the diversity of infrastructures with which downstream software must interact. | Standardization and access remain tied to a specific repository and data model rather than a common abstraction across independently designed resources. |
| Wang et al., **“Sharing and community curation of mass spectrometry data with Global Natural Products Social Molecular Networking,”** *Nature Biotechnology*, 2016. [Nature](https://www.nature.com/articles/nbt.3597) | Demonstrates that shared infrastructure for MS/MS data enables community curation, molecular networking, data reuse, and collaborative analysis. | Shows the downstream scientific value created when previously fragmented mass-spectrometry data become accessible through shared infrastructure. | GNPS addresses sharing, analysis, and curation within a mass-spectrometry ecosystem rather than general dataset acquisition and programmatic interoperability. |
| Jarmusch et al., **“ReDU: a framework to find and reanalyze public mass spectrometry data,”** *Nature Methods*, 2020. [Nature](https://www.nature.com/articles/s41592-020-0916-7) | Demonstrates that controlled metadata normalization across public mass-spectrometry datasets enables cross-study discovery, filtering, and repository-scale reanalysis. | Closely supports the premise that structure added across independently produced datasets can create new downstream capabilities. | ReDU focuses on harmonizing mass-spectrometry data and metadata for search and reanalysis rather than providing a reusable programming abstraction over heterogeneous metabolomics resources more generally. |
| Akhtar et al., **“Croissant: A Metadata Format for ML-Ready Datasets,”** 2024. [arXiv](https://arxiv.org/abs/2403.19546) | Proposes a shared dataset description representing metadata, resources, structure, and information required to load datasets across tools and platforms. | Provides a cross-domain precedent for making independently distributed datasets programmatically usable through a shared machine-readable description. | Croissant is designed primarily around machine-learning datasets and does not address metabolomics-specific resources, data types, repositories, or scientific workflows. |

Together, these works show that interoperability has been addressed at several levels: FAIR defines general objectives; metabolomics standards address reporting and exchange formats; repositories standardize resources within particular ecosystems; GNPS and ReDU demonstrate the scientific value of harmonization; and Croissant demonstrates a shared dataset-description approach in another computational domain. The remaining question addressed here is narrower: whether the acquisition and dataset-description layer itself can be generalized across independently maintained metabolomics resources, and which information is required for that abstraction to remain scientifically useful. As well as if a single representation is possible and practical. 

## Expected Results

The expected results from this project include a library that ingests metabolomics datasets, applies a standard representation for the given dataset, including acquisition/materialization, and allows for documenting where and which properties can be generalized, and how. Where generalization fails, this is documented. 

Thus there are three expected outputs:
1. Scientific results: Which properties are required for interoperable programmatic representation of heterogeneous metabolomics datasets? 
2. Engineering artifact: the library itself (`metabolomics-datasets-rs`)
3. Empirical results: Set of real datasets representable through the abstraction, as well as documentation for failures and/or exceptions. 


## Similar Projects

| Project | What to investigate | Why it matters to `metabolomics-datasets-rs` |
|---|---|---|
| **Hugging Face `datasets`** | Dataset registry, metadata, caching, remote/local loading, streaming, dataset cards | Required by Luca and one of the clearest examples of `load_dataset(...)` as a common abstraction. Current docs describe loading local/Hub datasets, caching and Arrow-based processing. [Hugging Face](https://huggingface.co/docs/datasets/main/en/index) |
| **TensorFlow Datasets** | Per-dataset builders, downloading/preparation, versions, citations, metadata | **Very close architectural precedent.** `DatasetBuilder` encapsulates downloading/preparing a dataset plus documentation/version/features. [TensorFlow](https://www.tensorflow.org/datasets/overview) |
| **Bioconductor ExperimentHub** | Curated resources + metadata + transparent local caching | Another very close precedent, but for scientific data rather than generic ML. Current release explicitly describes local cache management for reproducible data access. [Bioconductor](https://bioconductor.org/packages/release/bioc/html/ExperimentHub.html) |
| **OpenML** | Searchable dataset registry, stable dataset IDs, API-based acquisition | Demonstrates dataset discovery + metadata + programmatic retrieval. [OpenML](https://www.openml.org/apis) |
| **DataLad** | Distributed dataset acquisition/versioning/publication | Useful comparator for provenance and decentralized datasets rather than centralized hosting. [DataLad](https://docs.datalad.org/en/latest/index.html) |
| **DVC** | Dataset/artifact versioning, remote storage, reproducibility | Particularly relevant to your problem with changing upstream dataset versions and reproducibility. [Data Version Control](https://doc.dvc.org/user-guide) |
| **Frictionless Data** | Data packages, resources, schemas, validation, URL-based resources | Very relevant to your emerging `Dataset`/`DatasetArtifact` distinction; a Data Package is metadata plus one or more resources and can point at remote files. [Frictionless Framework](https://framework.frictionlessdata.io/docs/framework/package.html) |
| **Croissant** | Shared metadata vocabulary + resource/file structures + loading semantics | Probably the strongest metadata-interoperability comparator. [MLCommons](https://docs.mlcommons.org/croissant/) |
| **RO-Crate** | JSON-LD research-object packaging and linked metadata | Important because it approaches interoperability from the FAIR/semantic-web side rather than through a programming library. The current long-term release is RO-Crate 1.3. [researchobject.org](https://www.researchobject.org/ro-crate/specification) |
| **torchvision.datasets** | Built-in dataset classes, downloading/extraction and custom dataset abstractions | Simpler precedent showing how a language library can normalize acquisition of independently hosted datasets. [PyTorch Docs](https://docs.pytorch.org/vision/main/datasets.html) |

## Scope

This project currently is focused on metabolomics datasets that have been made publicly available. However it is foreseeable that if the implementation is successful in creating a unified interface for working with metabolomics datasets that support be added for access to proprietary datasets. 

For technology, the project is planned to be written in Rust, leaning on Rust's type system to help ensure enforcement of interoperability rules. 

### Datasets

| Dataset | Content | Source | Approximate size | Access mechanism |
|---|---|---|---|---|
| LOTUS | ... | Zenodo | ... | Zenodo API |
| COCONUT | ... | ... | ... | ... |
| PubChem | ... | ... | ... | ... |

## Technologies

Rust will be used for the implementation, with typed representations for dataset metadata and acquisition sources. Initial access mechanisms include HTTP and repository APIs such as Zenodo. Development and testing require ordinary workstation compute and sufficient local storage to materialize the selected datasets. No specialized compute infrastructure is presently anticipated. Public datasets will be prioritized so that authentication or proprietary infrastructure is not required for the initial study.