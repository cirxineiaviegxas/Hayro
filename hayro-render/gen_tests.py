import os
import json
import hashlib
import requests
from pathlib import Path

class TestGenerator:
    def __init__(self):
        self.script_dir = Path(__file__).resolve().parent
        self.custom_manifest_path = self.script_dir / 'manifest.json'
        self.pdfjs_manifest_path = self.script_dir / 'manifest_pdfjs.json'
        self.pdfs_dir = self.script_dir / 'pdfs'
        self.downloads_dir = self.script_dir / 'downloads'
        self.output_file = self.script_dir / 'tests' / 'tests.rs'
        
    def ensure_downloads_dir(self):
        """Create downloads directory if it doesn't exist."""
        self.downloads_dir.mkdir(exist_ok=True)
        
    def calculate_md5(self, file_path: Path) -> str:
        """Calculate MD5 hash of a file."""
        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()
        
    def download_pdf(self, link_path: Path, expected_md5: str = None) -> bool:
        """Download PDF from link file and verify MD5 if provided."""
        stem = link_path.stem
        dest_path = self.downloads_dir / f"{stem}.pdf"
        
        # If file exists, check MD5
        if dest_path.exists():
            if expected_md5:
                actual_md5 = self.calculate_md5(dest_path)
                if actual_md5 == expected_md5:
                    # print(f"✔ {stem} exists with correct MD5")
                    return True
                else:
                    print(f"⚠ {stem} exists but MD5 mismatch. Re-downloading...")
                    print(f"  Expected: {expected_md5}")
                    print(f"  Actual:   {actual_md5}")
            else:
                print(f"✔ Skipping {stem} (already downloaded, no MD5 verification)")
                return True
        
        # Download the file
        url = link_path.read_text().strip()
        print(f"📥 Downloading {stem} from {url[:50]}...")
        
        try:
            response = requests.get(url, stream=True, timeout=30)
            response.raise_for_status()
            
            with open(dest_path, 'wb') as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)
            
            # Verify MD5 if provided
            if expected_md5:
                actual_md5 = self.calculate_md5(dest_path)
                if actual_md5 == expected_md5:
                    print(f"✔ Downloaded and verified MD5")
                    return True
                else:
                    print(f"✘ MD5 verification failed!")
                    print(f"  Expected: {expected_md5}")
                    print(f"  Actual:   {actual_md5}")
                    dest_path.unlink()  # Delete the incorrect file
                    return False
            else:
                print(f"✔ Downloaded (no MD5 verification)")
                return True
                
        except requests.RequestException as e:
            print(f"✘ Failed to download {stem}: {e}")
            return False
            
    def load_manifests(self) -> list:
        """Load and parse both manifest files, combining them."""
        all_entries = []
        
        # Load custom manifest
        if self.custom_manifest_path.exists():
            with open(self.custom_manifest_path, 'r') as f:
                custom_entries = json.load(f)
                all_entries.extend(custom_entries)
                print(f"📋 Loaded {len(custom_entries)} entries from custom manifest")
        else:
            print("⚠ Custom manifest not found, skipping")
            
        # Load PDF.js manifest
        if self.pdfjs_manifest_path.exists():
            with open(self.pdfjs_manifest_path, 'r') as f:
                pdfjs_entries = json.load(f)
                all_entries.extend(pdfjs_entries)
                print(f"📋 Loaded {len(pdfjs_entries)} entries from PDF.js manifest")
        else:
            print("⚠ PDF.js manifest not found, skipping")
            
        if not all_entries:
            raise FileNotFoundError("No manifest files found or all manifests are empty")
            
        return all_entries
            
    def process_entry(self, entry: dict) -> bool:
        """Process a single manifest entry, downloading if necessary."""
        entry_id = entry['id']
        file_path = entry['file']
        is_link = entry.get('link', False)
        is_ignored = entry.get('ignore', False)
        expected_md5 = entry.get('md5')
        
        if is_ignored:
            print(f"⏭ Skipping {entry_id} (ignored)")
            return False
            
        if is_link:
            link_path = self.pdfs_dir / file_path.replace('pdfs/', '')
            if not link_path.exists():
                print(f"✘ Link file not found: {link_path}")
                return False
                
            success = self.download_pdf(link_path, expected_md5)
            if not success:
                print(f"✘ Failed to download or verify {entry_id}")
                return False
        else:
            # Check if PDF file exists
            pdf_path = self.pdfs_dir / file_path.replace('pdfs/', '')
            if not pdf_path.exists():
                print(f"✘ PDF file not found: {pdf_path}")
                return False
                
        return True
        
    def generate_rust_function(self, entry: dict) -> str:
        """Generate Rust test function for a manifest entry."""
        entry_id = entry['id']
        is_link = entry.get('link', False)
        first_page = entry.get('first_page')
        last_page = entry.get('last_page')
        
        # Generate page range string if specified
        if first_page is not None and last_page is not None:
            # Both start and end specified: "3..=7"
            length = f'Some("{first_page}..={last_page}")'
        elif first_page is not None:
            # Only start specified: "3.."
            length = f'Some("{first_page}..")'
        elif last_page is not None:
            # Only end specified: "..=7"
            length = f'Some("..={last_page}")'
        else:
            # No page range specified
            length = "None"
            
        func_name = entry_id.replace('-', '_')
            
        return f"#[test] fn {func_name}() {{ run_test(\"{entry_id}\", {str(is_link).lower()}, {length}); }}"
        
    def generate_tests(self):
        """Main function to generate tests from manifest."""
        print("🚀 Starting test generation from manifest...")
        
        # Ensure downloads directory exists
        self.ensure_downloads_dir()
        
        # Load manifests
        try:
            manifest = self.load_manifests()
            print(f"📋 Combined total: {len(manifest)} entries")
        except Exception as e:
            print(f"✘ Failed to load manifests: {e}")
            return
            
        # Process all entries and generate Rust functions
        rust_functions = []
        processed_count = 0
        skipped_count = 0
        
        for entry in manifest:
            if self.process_entry(entry):
                rust_functions.append(self.generate_rust_function(entry))
                processed_count += 1
            else:
                skipped_count += 1
                
        # Write Rust test file
        try:
            with open(self.output_file, 'w') as f:
                f.write('use crate::run_test;\n\n')
                f.write('\n'.join(rust_functions))
                
            print(f"\n🎉 Generated {len(rust_functions)} Rust test functions")
            print(f"📄 Output written to: {self.output_file}")
            print(f"📊 Summary: {processed_count} processed, {skipped_count} skipped")
            
        except Exception as e:
            print(f"✘ Failed to write output file: {e}")

def main():
    generator = TestGenerator()
    generator.generate_tests()

if __name__ == '__main__':
    main()
