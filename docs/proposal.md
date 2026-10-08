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

[ReDU](https://www.nature.com/articles/s41592-020-0916-7) - 

## Expected Results

## Similar Projects

## Scope

### Technologies

### Datasets 

### 