# Proposal

## Question

Does collecting and adding structure to separate metabolomics datasets with rust increase interoperability and produce an actionable artifact for downstream use?

## Insights

This project should lead to:
1. Documentation of the current state of metabolomics datasets' interoperability
2. Fundamental aspects necessary to making a dataset interoperable
3. Challenges and existing problems with unifying datasets in an interoperable manner. 

Impact on Commons Lab: 
The positive impacts of this project on the lab research can be categorized as: 
*Downstream Application*: Having ready access to a diverse set of datasets, organized by features, could enhance projects that require access to datasets. Moreover, given the majority of the lab's code is currently being written in Rust, a crate makes importing the datasets for use through a standardized method simple and clear. This also enhances the interoperability of the datasets and would allow for downstream use and filtering. Moreover, moving the abstraction for commonly used datasets, such as LOTUS, COCONUT, PUBChEM, to an upstream crate would easy their import for use with the relevant data formats (in this example SMILES). Rather than as a convenience feature flag on a parser crate. 

*Dataset creation and dataset usage*: The project also has the potential to help shape how the lab chooses to collect and store datasets, as insights from this project offers insights into interoperability from a practical standpoint. The project has the potential to identify common pain points for dataset use, including access issues already discovered, such as dynamic dataset links based on versions. Moreover, the practical access to a wide set of datasets and a standard way to ingest a new dataset into the existing structure presented in this project, should offer cleaner ways to interact with these databases, discovery of other relevant databases that may otherwise have been overlooked, and as a result present a richer data analysis environment for lab research. 

## Relevant Work

| Work | Relevance | Explanation | Gap |
|---|---|---|---|
| Wilkinson et al., **“The FAIR Guiding Principles for scientific data management and stewardship”**, *Scientific Data*, 2016 | The conceptual foundation | FAIR explicitly includes machine-actionable findability, access, interoperability and reuse. [Nature](https://www.nature.com/articles/sdata201618) | FAIR says what good data infrastructure should achieve, but does not prescribe a metabolomics implementation. |
| Hoffmann et al., **mzTab-M: A Data Standard for Sharing Quantitative Results in Mass Spectrometry Metabolomics**, *Analytical Chemistry*, 2019 | Metabolomics-specific interoperability | mzTab-M arose because heterogeneous metabolomics software outputs make sharing and reanalysis difficult; it standardizes quantitative MS metabolomics output with controlled vocabulary. [PubMed](https://pubmed.ncbi.nlm.nih.gov/30688441/) | Standardizes a particular representation, does not do discovery/acquisition of heterogeneous external datasets. |
| Haug et al., **MetaboLights: a resource evolving in response to the needs of its scientific community**, *Nucleic Acids Research*, 2020 | Major metabolomics repository | Example of a mature domain repository evolving around metabolomics data and metadata requirements. [OUP Academic](https://academic.oup.com/nar/article/48/D1/D440/5613675) | One repository/ecosystem; it does not give applications a common abstraction over MetaboLights, Zenodo, PubChem, COCONUT, other |
| Sud et al., **Metabolomics Workbench: An international repository for metabolomics data and metadata, metabolite standards, protocols, tutorials and training, and analysis tools**, *Nucleic Acids Research*, 2016 | Second major repository | Provides experimental data, metadata, standards, protocols and analysis resources in a public metabolomics repository. [OUP Academic](https://academic.oup.com/nar/article/44/D1/D463/2502588) | Valuable internal standardization, but repository-specific access and representation remain. |
| Jarmusch et al., **ReDU: a framework to find and reanalyze public mass spectrometry data**, *Nature Methods*, 2020 | Major framework for analyzing publicly available MS data | Demonstrates that controlled metadata normalization over public MS data enables cross-study discovery and repository-scale reanalysis. [Nature](https://www.nature.com/articles/s41592-020-0916-7) | ReDU is focused on public mass-spectrometry data and metadata harmonization; does not address whether heterogeneous metabolomics datasets can be represented and accessed through a reusable programming abstraction. |
| Wang et al., **Sharing and community curation of mass spectrometry data with Global Natural Products Social Molecular Networking**, *Nature Biotechnology*, 2016 | Demonstrates potential downstream payoff | GNPS built infrastructure for sharing and community curation of raw/processed/identified MS/MS data because existing sharing practices were inadequate. [Nature](https://www.nature.com/articles/nbt.3597) | GNPS solves a particular analysis/sharing ecosystem rather than general heterogeneous dataset acquisition/interoperability/application. |
| Akhtar et al., **Croissant: A Metadata Format for ML-Ready Datasets**, 2024 | Excellent cross-domain comparator | Croissant explicitly tries to create a shared dataset representation across tools/platforms, combining metadata with information needed to load data. [arXiv](https://arxiv.org/abs/2403.19546) | ML-oriented rather than metabolomics-specific—but philosophically extremely close to this project's goals. |

## Expected Results

## Similar Projects

## Scope

### Technologies

### Datasets 

### 