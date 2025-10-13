# OctaIndex3D Documentation

This directory contains documentation for the OctaIndex3D project.

## Whitepaper

The comprehensive technical whitepaper is available in the repository root:
- **Markdown**: [`../WHITEPAPER.md`](../WHITEPAPER.md) (version-controlled, GitHub-rendered)
- **PDF**: `whitepaper.pdf` (generated from Markdown)

### Generating the PDF

You can generate the PDF from the Markdown source using any of these methods:

#### Method 1: Pandoc (Recommended)

```bash
# Install pandoc (if not already installed)
# macOS: brew install pandoc basictex
# Ubuntu: sudo apt install pandoc texlive
# Windows: choco install pandoc miktex

# Generate PDF
pandoc ../WHITEPAPER.md \
    -o whitepaper.pdf \
    --pdf-engine=xelatex \
    -V geometry:margin=1in \
    -V fontsize=11pt \
    -V documentclass=article \
    --toc \
    --number-sections \
    --highlight-style=tango
```

#### Method 2: Grip + wkhtmltopdf

```bash
# Install grip and wkhtmltopdf
pip install grip
# macOS: brew install wkhtmltopdf
# Ubuntu: sudo apt install wkhtmltopdf

# Generate PDF
grip ../WHITEPAPER.md --export whitepaper.html
wkhtmltopdf whitepaper.html whitepaper.pdf
```

#### Method 3: Online Tools

- [Markdown to PDF](https://www.markdowntopdf.com/)
- [Dillinger](https://dillinger.io/) (export as PDF)
- GitHub's "Print to PDF" from rendered Markdown

#### Method 4: VS Code Extension

Install "Markdown PDF" extension:
1. Open `WHITEPAPER.md` in VS Code
2. Right-click → "Markdown PDF: Export (pdf)"

### Automated PDF Generation

The repository includes a GitHub Action that automatically generates the PDF on changes to `WHITEPAPER.md`. The PDF is attached to releases and can be downloaded from the Releases page.

## Citation

If you use OctaIndex3D in your research, please cite:

```bibtex
@techreport{mclarney2025octaindex3d,
  title={OctaIndex3D: A High-Performance 3D Spatial Indexing System Based on Body-Centered Cubic Lattice},
  author={McLarney, Michael A. and Claude},
  year={2025},
  institution={GitHub},
  url={https://github.com/FunKite/OctaIndex3D}
}
```

## Additional Documentation

- [API Documentation](https://docs.rs/octaindex3d) - Generated from source code
- [Examples](../examples/) - Code examples and tutorials
- [README](../README.md) - Quick start and overview
