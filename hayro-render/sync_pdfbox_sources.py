#!/usr/bin/env python3

import json
import hashlib
import requests
import shutil
from pathlib import Path
from urllib.parse import unquote

class PdfboxSourceSync:
    def __init__(self):
        self.script_dir = Path(__file__).resolve().parent
        self.sources_path = self.script_dir / 'pdfbox_sources.json'
        self.manifest_path = self.script_dir / 'manifest_pdfbox.json'
        self.pdfs_dir = self.script_dir / 'pdfs' / 'pdfbox'
        self.downloads_dir = self.script_dir / 'downloads' / 'pdfbox'
        
        # Ensure directories exist
        self.pdfs_dir.mkdir(parents=True, exist_ok=True)
        self.downloads_dir.mkdir(parents=True, exist_ok=True)
        
    def calculate_md5(self, file_path: Path) -> str:
        """Calculate MD5 hash of a file."""
        hash_md5 = hashlib.md5()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_md5.update(chunk)
        return hash_md5.hexdigest()
        
    def download_pdf(self, url: str, dest_path: Path) -> bool:
        """Download PDF from URL."""
        if dest_path.exists():
            print(f"✔ {dest_path.name} already exists, skipping download")
            return True
            
        print(f"📥 Downloading {dest_path.name} from {url[:50]}...")
        
        try:
            response = requests.get(url, stream=True, timeout=60)
            response.raise_for_status()
            
            with open(dest_path, 'wb') as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)
            
            print(f"✔ Downloaded {dest_path.name}")
            return True
            
        except requests.RequestException as e:
            print(f"✘ Failed to download {dest_path.name}: {e}")
            if dest_path.exists():
                dest_path.unlink()  # Clean up partial download
            return False
            
    def create_link_file(self, link_path: Path, url: str):
        """Create a .link file with the given URL."""
        if link_path.exists():
            existing_url = link_path.read_text().strip()
            if existing_url == url:
                print(f"✔ {link_path.name} already exists with correct URL")
                return
            else:
                print(f"⚠ {link_path.name} exists but URL differs, updating...")
        
        link_path.write_text(url)
        print(f"✔ Created {link_path.name}")
        
    def generate_test_name(self, issue: str, index: int, total: int) -> str:
        """Generate test name for the given issue and index."""
        if total == 1:
            return issue
        else:
            return f"{issue}_{index + 1}"
            
    def sync(self):
        """Main synchronization function."""
        print("🚀 Starting PDFBOX source synchronization...")
        
        # Load sources
        if not self.sources_path.exists():
            print(f"✘ Sources file not found: {self.sources_path}")
            return
            
        with open(self.sources_path, 'r') as f:
            sources = json.load(f)
        
        print(f"📋 Found {len(sources)} PDFBOX issues with {sum(len(urls) for urls in sources.values())} total PDFs")
        
        manifest_entries = []
        processed_count = 0
        failed_count = 0
        
        for issue, urls in sources.items():
            print(f"\n📦 Processing PDFBOX-{issue} ({len(urls)} PDFs)...")
            
            for i, url in enumerate(urls):
                test_name = self.generate_test_name(issue, i, len(urls))
                
                # Create .link file
                link_path = self.pdfs_dir / f"{test_name}.link"
                self.create_link_file(link_path, url)
                
                # Download PDF
                pdf_path = self.downloads_dir / f"{test_name}.pdf"
                if self.download_pdf(url, pdf_path):
                    # Calculate MD5
                    md5_hash = self.calculate_md5(pdf_path)
                    print(f"🔢 MD5 for {test_name}: {md5_hash}")
                    
                    # Add to manifest
                    manifest_entry = {
                        "id": test_name,
                        "file": f"pdfs/{test_name}.link",
                        "md5": md5_hash,
                        "link": True
                    }
                    manifest_entries.append(manifest_entry)
                    processed_count += 1
                else:
                    print(f"✘ Failed to process {test_name}")
                    failed_count += 1
                    # Clean up link file if download failed
                    if link_path.exists():
                        link_path.unlink()
        
        # Sort manifest entries by ID for consistency
        manifest_entries.sort(key=lambda x: (int(x['id'].split('_')[0]), x['id']))
        
        # Write manifest
        with open(self.manifest_path, 'w') as f:
            json.dump(manifest_entries, f, indent=2)
            
        print(f"\n🎉 Synchronization complete!")
        print(f"📄 Generated {self.manifest_path} with {len(manifest_entries)} entries")
        print(f"📊 Summary: {processed_count} successful, {failed_count} failed")
        
        if failed_count > 0:
            print(f"⚠ {failed_count} entries failed. Check URLs and try again.")
            
    def cleanup_removed_entries(self):
        """Remove files for entries that are no longer in the sources."""
        if not self.sources_path.exists():
            return
            
        with open(self.sources_path, 'r') as f:
            sources = json.load(f)
            
        # Generate expected test names
        expected_names = set()
        for issue, urls in sources.items():
            for i in range(len(urls)):
                test_name = self.generate_test_name(issue, i, len(urls))
                expected_names.add(test_name)
        
        removed_count = 0
        
        # Clean up link files
        for link_file in self.pdfs_dir.glob("*.link"):
            test_name = link_file.stem
            if test_name not in expected_names:
                link_file.unlink()
                print(f"🧹 Removed {link_file.name}")
                removed_count += 1
                
        # Clean up downloaded PDFs
        for pdf_file in self.downloads_dir.glob("*.pdf"):
            test_name = pdf_file.stem
            if test_name not in expected_names:
                pdf_file.unlink()
                print(f"🧹 Removed {pdf_file.name}")
                removed_count += 1
                
        if removed_count > 0:
            print(f"🧹 Cleaned up {removed_count} obsolete files")

def main():
    syncer = PdfboxSourceSync()
    
    # Clean up first
    syncer.cleanup_removed_entries()
    
    # Then sync
    syncer.sync()

if __name__ == '__main__':
    main() 