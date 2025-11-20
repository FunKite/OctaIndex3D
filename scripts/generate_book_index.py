#!/usr/bin/env python3
import os
import re
from collections import defaultdict

BOOK_DIR = "book"
INDEX_FILE = os.path.join(BOOK_DIR, "back_matter", "index.md")
IGNORE_DIRS = ["images", "back_matter"]
IGNORE_FILES = ["index.md", "SUMMARY.md", "BOOK_ENHANCEMENT_SUGGESTIONS.md", "ERRATA.md", "LICENSE.md"]

# Terms to explicitly index (can be expanded)
EXPLICIT_TERMS = [
    "BCC", "Body-Centered Cubic", "Octree", "Morton", "Hilbert", "SFC", 
    "Isotropy", "Anisotropy", "Voronoi", "Truncated Octahedron", 
    "Parity", "Coordinate", "Lattice", "Sampling", "Nyquist",
    "Galactic128", "Index64", "Route64", "Hilbert64", "Frame Registry",
    "Container", "Streaming", "Compression", "Delta Encoding",
    "SIMD", "AVX2", "NEON", "BMI2", "GPU", "CUDA", "Metal", "Vulkan",
    "Pathfinding", "A*", "Dijkstra", "Occupancy Grid", "SLAM",
    "Geospatial", "WGS84", "ECEF", "ENU", "GIS",
    "Molecular Dynamics", "Crystallography", "CFD",
    "Voxel", "LOD", "Level of Detail", "Procedural Generation",
    "Distributed", "Sharding", "Arrow", "Parquet",
    "Machine Learning", "GNN", "Point Cloud", "PyTorch"
]

def scan_files():
    index = defaultdict(list)
    
    for root, dirs, files in os.walk(BOOK_DIR):
        # Filter directories
        dirs[:] = [d for d in dirs if d not in IGNORE_DIRS]
        
        for file in files:
            if not file.endswith(".md") or file in IGNORE_FILES:
                continue
                
            path = os.path.join(root, file)
            rel_path = os.path.relpath(path, BOOK_DIR)
            
            # Determine chapter/section name from filename or content
            title = get_title(path)
            link_path = rel_path
            
            with open(path, 'r', encoding='utf-8') as f:
                content = f.read()
                
            # Scan for terms
            for term in EXPLICIT_TERMS:
                # Case-insensitive search, but store as the canonical term
                if re.search(r'\b' + re.escape(term) + r'\b', content, re.IGNORECASE):
                    index[term].append((title, link_path))

    return index

def get_title(path):
    with open(path, 'r', encoding='utf-8') as f:
        for line in f:
            if line.startswith("# "):
                return line.strip("# ").strip()
    return os.path.basename(path)

def generate_index_md(index):
    sorted_terms = sorted(index.keys(), key=lambda s: s.lower())
    
    md_content = "# Index\n\n"
    
    current_letter = ""
    
    for term in sorted_terms:
        letter = term[0].upper()
        if letter != current_letter:
            md_content += f"\n## {letter}\n\n"
            current_letter = letter
            
        links = index[term]
        # Deduplicate links based on title
        unique_links = {}
        for title, path in links:
            unique_links[title] = path
            
        links_md = ", ".join([f"[{title}](../{path})" for title, path in unique_links.items()])
        md_content += f"- **{term}**: {links_md}\n"
        
    return md_content

def main():
    print(f"Scanning book content in {BOOK_DIR}...")
    index_data = scan_files()
    print(f"Found {len(index_data)} terms.")
    
    content = generate_index_md(index_data)
    
    os.makedirs(os.path.dirname(INDEX_FILE), exist_ok=True)
    with open(INDEX_FILE, 'w', encoding='utf-8') as f:
        f.write(content)
        
    print(f"Index generated at {INDEX_FILE}")

if __name__ == "__main__":
    main()
