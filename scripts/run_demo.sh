#!/usr/bin/env bash
# Открывает scripts/negative_edges_demo.ipynb в эфемерном окружении uv.
# В систему ничего не устанавливается: uv собирает venv в своём кэше и не трогает
# ни системный python, ни ~/.local. Нужен только сам uv.
set -euo pipefail

cd "$(dirname "$0")/.."

exec uv run --no-project \
  --with matplotlib \
  --with ipywidgets \
  --with jupyterlab \
  jupyter lab scripts/negative_edges_demo.ipynb "$@"
