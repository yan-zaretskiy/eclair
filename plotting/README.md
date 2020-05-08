## Building

To enable the Jupyter-based plotting of the UNSMRY data:

1. Install `conda` from https://www.anaconda.com/products/individual.
2. Run 
```
conda env create --file environment.yml
conda activate eclair
python -m ipykernel install --user --name=eclair
```

After that navigate to the `eclpy` folder of this repo and run
```
maturin develop
```
At this point you can go back to the `plotting` folder and start the Jupyter notebook server
```
jupyter notebook
```