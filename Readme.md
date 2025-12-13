# Peg Solitaire Solver

The classic single player game of Peg Solitaire, implemented in Rust for the browser.

This project implements an automated solver that is optimized for the constraints relevant
to a WASM application: low RAM and network usage.

## Project Structure

* `frontend` contains the Rust code for rendering the game as a web application.
* `precompute` contains Rust code for computing the bloom filters and for running performance evaluations.
* `common` is a Rust library crate containing some shared game logic code.
* `report` contains the typst source for the [paper](https://projects.pascalsommer.ch/pegsolitaire/precomputing-pegsolitaire-paper.pdf) explaining the method.
* `evaluation` contains some Jupyter notebooks to analyze the measurements and generate plots for the report.
