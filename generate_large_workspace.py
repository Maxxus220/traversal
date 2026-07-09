#!/usr/bin/env python3
import os
import shutil
import random
import argparse

# List of directories to create
DIRS = [
    "src",
    "src/components",
    "src/utils",
    "src/services",
    "src/models",
    "include",
    "lib",
    "docs",
    "tests",
    "scripts",
]

# File types and their comment formats
FILE_TYPES = [
    {
        "ext": "rs",
        "comment": "// {}",
        "templates": [
            "fn function_{}() {{\n    let x = {};\n}}",
            "struct Struct_{} {{\n    field: {},\n}}",
            "impl Struct_{} {{\n    fn new() -> Self {{\n        Self {{ field: {} }}\n    }}\n}}",
        ]
    },
    {
        "ext": "cpp",
        "comment": "// {}",
        "templates": [
            "void function_{}() {{\n    int x = {};\n}}",
            "class Class_{} {{\npublic:\n    int field_{};\n}};",
        ]
    },
    {
        "ext": "h",
        "comment": "// {}",
        "templates": [
            "#ifndef HEADER_{}_H\n#define HEADER_{}_H\n#endif",
            "void function_{}(int x);",
        ]
    },
    {
        "ext": "py",
        "comment": "# {}",
        "templates": [
            "def function_{}():\n    x = {}\n    return x",
            "class Class_{}:\n    def __init__(self):\n        self.field = {}",
        ]
    },
    {
        "ext": "js",
        "comment": "// {}",
        "templates": [
            "function function_{}() {{\n    const x = {};\n    return x;\n}}",
            "class Class_{} {{\n    constructor() {{\n        this.field = {};\n    }}\n}}",
        ]
    },
    {
        "ext": "ts",
        "comment": "// {}",
        "templates": [
            "function function_{}(param: number): void {{\n    const x: number = {};\n}}",
            "interface Interface_{} {{\n    field: number;\n}}",
        ]
    },
    {
        "ext": "go",
        "comment": "// {}",
        "templates": [
            "func function_{}() {{\n\tx := {}\n\t_ = x\n}}",
            "type Struct_{} struct {{\n\tField int\n}}",
        ]
    },
    {
        "ext": "java",
        "comment": "// {}",
        "templates": [
            "public class Class_{} {{\n    public void method() {{\n        int x = {};\n    }}\n}}",
        ]
    },
    {
        "ext": "md",
        "comment": "<!-- {} -->",
        "templates": [
            "# Section {}\n\nThis is some description about item {}.\n",
            "## Subsection {}\n\nDetails about {}.\n",
        ]
    },
    {
        "ext": "html",
        "comment": "<!-- {} -->",
        "templates": [
            "<div id=\"div_{}\">\n    <p>Paragraph {}</p>\n</div>",
            "<section class=\"section_{}\">\n    <h1>Heading {}</h1>\n</section>",
        ]
    },
    {
        "ext": "css",
        "comment": "/* {} */",
        "templates": [
            ".class_{} {{\n    color: #{};\n}}",
            "#id_{} {{\n    margin: {}px;\n}}",
        ]
    }
]

def main():
    parser = argparse.ArgumentParser(description="Generate a large test workspace with traversal tags.")
    parser.add_argument("--num-files", type=int, default=1000, help="Number of files to generate")
    parser.add_argument("--lines-per-file", type=int, default=150, help="Approximate lines per file")
    parser.add_argument("--num-tags", type=int, default=2000, help="Number of tags to generate")
    parser.add_argument("--output-dir", type=str, default="test_workspace_large", help="Output directory name")
    args = parser.parse_args()

    output_path = os.path.abspath(args.output_dir)
    print(f"Generating large workspace at: {output_path}")

    # Remove existing directory if it exists
    if os.path.exists(output_path):
        print(f"Removing existing directory: {output_path}")
        shutil.rmtree(output_path)

    os.makedirs(output_path)

    # Create subdirectories
    for d in DIRS:
        os.makedirs(os.path.join(output_path, d), exist_ok=True)

    # Generate tag definitions
    tags = [f"tag_{i:04d}" for i in range(args.num_tags)]

    # We will distribute files across directories
    files_metadata = []
    for i in range(args.num_files):
        d = random.choice(DIRS)
        ft = random.choice(FILE_TYPES)
        filename = f"file_{i:04d}.{ft['ext']}"
        filepath = os.path.join(d, filename)
        files_metadata.append({
            "filepath": filepath,
            "ft": ft,
            "targets": [],
            "links": []
        })

    # Distribute targets: each tag must have exactly one target in some file
    for tag in tags:
        file_meta = random.choice(files_metadata)
        file_meta["targets"].append(tag)

    # Distribute links: each tag can have 0 to 3 links in other files
    for tag in tags:
        num_links = random.randint(0, 3)
        for _ in range(num_links):
            file_meta = random.choice(files_metadata)
            file_meta["links"].append(tag)

    # Write files
    for idx, file_meta in enumerate(files_metadata):
        full_path = os.path.join(output_path, file_meta["filepath"])
        ft = file_meta["ft"]
        comment_tmpl = ft["comment"]
        
        # Prepare targets and links lines
        target_lines = [comment_tmpl.format(f"[traverse-tgt: {t}]") for t in file_meta["targets"]]
        link_lines = [comment_tmpl.format(f"[traverse-lnk: {l}]") for l in file_meta["links"]]
        
        all_tag_lines = target_lines + link_lines
        random.shuffle(all_tag_lines)

        # Generate fake contents
        lines = []
        approx_templates = args.lines_per_file // 5 # each template block is ~5 lines
        if approx_templates < 1:
            approx_templates = 1
            
        for t_idx in range(approx_templates):
            tmpl = random.choice(ft["templates"])
            # Fill template placeholders with random numbers
            val1 = random.randint(100, 999)
            val2 = random.randint(1000, 9999)
            if tmpl.count("{}") == 1:
                lines.append(tmpl.format(val1))
            elif tmpl.count("{}") == 2:
                lines.append(tmpl.format(val1, val2))
            else:
                lines.append(tmpl)

        # Insert tags at random lines
        for tag_line in all_tag_lines:
            insert_idx = random.randint(0, len(lines))
            lines.insert(insert_idx, tag_line)

        # Write to file
        with open(full_path, "w", encoding="utf-8") as f:
            f.write("\n\n".join(lines))
            f.write("\n")

    print(f"Successfully generated {args.num_files} files with {args.num_tags} tags in {output_path}.")

if __name__ == "__main__":
    main()
