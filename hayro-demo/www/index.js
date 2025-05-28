import init, { PdfViewer } from './hayro_demo.js';

let pdfViewer = null;
let currentPage = 1;
let totalPages = 0;
let currentScale = 2.0;

async function run() {
    await init();
    
    const dropZone = document.getElementById('drop-zone');
    const fileInput = document.getElementById('file-input');
    const fileSelector = document.getElementById('file-selector');
    const viewer = document.getElementById('viewer');
    const canvas = document.getElementById('pdf-canvas');
    const prevButton = document.getElementById('prev-page');
    const nextButton = document.getElementById('next-page');
    const pageInfo = document.getElementById('page-info');
    const pageInput = document.getElementById('page-input');
    const zoomOutButton = document.getElementById('zoom-out');
    const zoomInButton = document.getElementById('zoom-in');
    const zoomInfo = document.getElementById('zoom-info');

    // Make drop zone clickable to open file dialog
    dropZone.addEventListener('click', () => {
        fileInput.click();
    });

    // Comprehensive drag and drop prevention
    const preventDefaults = (e) => {
        e.preventDefault();
        e.stopPropagation();
    };

    // Prevent default drag behaviors on the entire document
    ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
        document.addEventListener(eventName, preventDefaults, false);
        document.body.addEventListener(eventName, preventDefaults, false);
    });

    // Highlight drop zone when item is dragged over it
    ['dragenter', 'dragover'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            preventDefaults(e);
            dropZone.classList.add('dragover');
        }, false);
    });

    ['dragleave', 'drop'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            preventDefaults(e);
            dropZone.classList.remove('dragover');
        }, false);
    });

    // Handle dropped files
    dropZone.addEventListener('drop', (e) => {
        preventDefaults(e);
        const dt = e.dataTransfer;
        const files = dt.files;
        
        if (files.length > 0) {
            handleFile(files[0]);
        }
    }, false);

    // Handle file selection
    fileInput.addEventListener('change', (e) => {
        if (e.target.files.length > 0) {
            handleFile(e.target.files[0]);
        }
    });

    // Navigation handlers
    prevButton.addEventListener('click', () => {
        if (currentPage > 1) {
            currentPage--;
            renderCurrentPage();
        }
    });

    nextButton.addEventListener('click', () => {
        if (currentPage < totalPages) {
            currentPage++;
            renderCurrentPage();
        }
    });

    // Zoom handlers
    zoomOutButton.addEventListener('click', () => {
        if (currentScale > 0.5) {
            currentScale = Math.max(0.5, currentScale - 0.25);
            updateZoomInfo();
            renderCurrentPage();
        }
    });

    zoomInButton.addEventListener('click', () => {
        if (currentScale < 3.0) {
            currentScale = Math.min(3.0, currentScale + 0.25);
            updateZoomInfo();
            renderCurrentPage();
        }
    });

    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => {
        if (!pdfViewer) return;
        
        switch(e.key) {
            case 'ArrowLeft':
                if (currentPage > 1) {
                    currentPage--;
                    renderCurrentPage();
                }
                break;
            case 'ArrowRight':
                if (currentPage < totalPages) {
                    currentPage++;
                    renderCurrentPage();
                }
                break;
            case '+':
            case '=':
                if (currentScale < 3.0) {
                    currentScale = Math.min(3.0, currentScale + 0.25);
                    updateZoomInfo();
                    renderCurrentPage();
                }
                break;
            case '-':
                if (currentScale > 0.5) {
                    currentScale = Math.max(0.5, currentScale - 0.25);
                    updateZoomInfo();
                    renderCurrentPage();
                }
                break;
            case 'Escape':
                // Go back to file selection
                resetToFileSelection();
                break;
        }
    });

    // Page input handler
    pageInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            const targetPage = parseInt(pageInput.value);
            if (targetPage >= 1 && targetPage <= totalPages) {
                currentPage = targetPage;
                renderCurrentPage();
                pageInput.blur(); // Remove focus from input
            } else {
                // Reset to current page if invalid
                pageInput.value = currentPage;
            }
        } else if (e.key === 'Escape') {
            // Reset and blur on escape
            pageInput.value = currentPage;
            pageInput.blur();
        }
    });

    pageInput.addEventListener('blur', () => {
        // Reset to current page when losing focus
        pageInput.value = currentPage;
    });

    async function handleFile(file) {
        if (file.type !== 'application/pdf') {
            alert('Please select a PDF file');
            return;
        }

        try {
            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            
            pdfViewer = new PdfViewer();
            await pdfViewer.load_pdf(uint8Array);
            
            totalPages = pdfViewer.get_page_count();
            currentPage = 1;
            currentScale = 2.0;
            
            fileSelector.style.display = 'none';
            viewer.style.display = 'flex';
            
            // Set max attribute for page input
            pageInput.setAttribute('max', totalPages);
            pageInput.value = currentPage;
            
            updateZoomInfo();
            renderCurrentPage();
            
        } catch (error) {
            console.error('Error loading PDF:', error);
            alert('Error loading PDF: ' + error.message);
        }
    }

    async function renderCurrentPage() {
        if (!pdfViewer) return;

        try {
            // Set the current page in the Rust viewer
            if (currentPage > 1) {
                // Reset to page 1 first, then navigate to desired page
                while (pdfViewer.get_current_page() > 1) {
                    pdfViewer.previous_page();
                }
                // Navigate to the desired page
                for (let i = 1; i < currentPage; i++) {
                    pdfViewer.next_page();
                }
            } else {
                // Reset to page 1
                while (pdfViewer.get_current_page() > 1) {
                    pdfViewer.previous_page();
                }
            }
            
            const imageData = await pdfViewer.render_current_page(currentScale);
            
            const blob = new Blob([imageData], { type: 'image/png' });
            const url = URL.createObjectURL(blob);
            
            const img = new Image();
            img.onload = () => {
                const ctx = canvas.getContext('2d');
                canvas.width = img.width;
                canvas.height = img.height;
                ctx.drawImage(img, 0, 0);
                URL.revokeObjectURL(url);
            };
            img.src = url;
            
            updatePageInfo();
            updateNavigationButtons();
            
        } catch (error) {
            console.error('Error rendering page:', error);
            alert('Error rendering page: ' + error.message);
        }
    }

    function updatePageInfo() {
        pageInfo.textContent = `${currentPage} / ${totalPages}`;
        pageInput.value = currentPage;
    }

    function updateZoomInfo() {
        zoomInfo.textContent = `${Math.round(currentScale * 100)}%`;
    }

    function updateNavigationButtons() {
        prevButton.disabled = currentPage <= 1;
        nextButton.disabled = currentPage >= totalPages;
        zoomOutButton.disabled = currentScale <= 0.5;
        zoomInButton.disabled = currentScale >= 3.0;
    }

    function resetToFileSelection() {
        viewer.style.display = 'none';
        fileSelector.style.display = 'flex';
        pdfViewer = null;
        currentPage = 1;
        totalPages = 0;
        currentScale = 2.0;
        fileInput.value = '';
    }
}

run().catch(console.error); 