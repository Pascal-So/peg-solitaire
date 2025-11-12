import matplotlib
import matplotlib.pyplot as plt
import seaborn as sns

colors = sns.color_palette("husl", 6)
category_colors = {
    "prime": colors[3],
    "round": colors[0],
    "round_minus_one": colors[2],
}
category_markers = {
    "prime": "*",
    "round": "x",
    "round_minus_one": "o",
}

def setup_plotting_style():
    matplotlib.style.use("ggplot")
    matplotlib.rcParams['figure.dpi'] = 300
    plt.rcParams.update({
        "text.usetex": True,
        "font.family": "serif"
    })

def setup_bytes_formatters(axis, minor = False):
    axis.set_major_formatter(plt.FuncFormatter(bytes_to_human_readable))
    if minor:
        axis.set_minor_formatter(plt.FuncFormatter(bytes_to_human_readable))
    else:
        axis.set_minor_formatter(plt.NullFormatter())

def bytes_to_human_readable(x, pos):
    if x < 1024:
        return "{:.0f}B".format(x)
    elif x < 1024**2:
        return "{:.0f}KiB".format(x / 1024)
    else:
        return "{:.0f}MiB".format(x / 1024**2)
