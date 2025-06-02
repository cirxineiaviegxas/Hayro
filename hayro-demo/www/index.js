import init, { PdfViewer } from './hayro_demo.js';

let pdfViewer = null;
let currentPage = 1;
let totalPages = 0;
let currentScale = 2.0;
let imageX = 0; // Image position relative to canvas center
let imageY = 0;
let isPanning = false;
let lastMouseX = 0;
let lastMouseY = 0;

// High-resolution rendering variables
let currentRenderScale = 2.0; // The scale at which the current image was rendered
let highResTimeout = null; // Timer for debounced high-res rendering

// Get the loading indicator from HTML
const loadingIndicator = document.getElementById('loading-indicator');

// Get the render time indicator from HTML
const renderTimeIndicator = document.getElementById('render-time-indicator');
const renderTimeText = document.getElementById('render-time-text');

// Render timing variables
let renderStartTime = 0;

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

    // Functions to show/hide loading indicator
    function showLoading() {
        console.log('Showing loading spinner');
        loadingIndicator.style.display = 'block';
    }

    function hideLoading() {
        console.log('Hiding loading spinner');
        loadingIndicator.style.display = 'none';
    }

    // Functions to update render time
    function startRenderTimer() {
        renderStartTime = performance.now();
    }

    function updateRenderTime() {
        const renderTime = performance.now() - renderStartTime;
        renderTimeText.textContent = `${Math.round(renderTime)}ms`;
        renderTimeIndicator.classList.add('visible');
        
        // Hide the indicator after 3 seconds
        setTimeout(() => {
            renderTimeIndicator.classList.remove('visible');
        }, 3000);
    }

    // Try to load cached PDF on startup
    loadCachedPDF();

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

    // Function to handle PDF files from drag and drop or file input
    function handlePDFDrop(e, files) {
        preventDefaults(e);
        
        // Remove dragover styling from both drop zone and viewer
        dropZone.classList.remove('dragover');
        viewer.classList.remove('dragover');
        
        if (files.length > 0) {
            handleFile(files[0]);
        }
    }

    // Highlight drop zones when item is dragged over them
    ['dragenter', 'dragover'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            preventDefaults(e);
            dropZone.classList.add('dragover');
        }, false);
        
        // Also allow dropping on the viewer when a PDF is open
        viewer.addEventListener(eventName, (e) => {
            preventDefaults(e);
            viewer.classList.add('dragover');
        }, false);
    });

    ['dragleave', 'drop'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            preventDefaults(e);
            dropZone.classList.remove('dragover');
        }, false);
        
        viewer.addEventListener(eventName, (e) => {
            preventDefaults(e);
            viewer.classList.remove('dragover');
        }, false);
    });

    // Handle dropped files on both drop zone and viewer
    dropZone.addEventListener('drop', (e) => {
        handlePDFDrop(e, e.dataTransfer.files);
    }, false);

    viewer.addEventListener('drop', (e) => {
        handlePDFDrop(e, e.dataTransfer.files);
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

    // Helper function for zooming
    function applyZoom(newScale) {
        if (!currentImage) return false;

        if (newScale < 0.5 || newScale > 40.0) return false;

        currentScale = newScale;

        updateZoomInfo();
        updateCursorStyle();
        renderCurrentPage(false);
        
        // Schedule high-resolution re-render after 1.5 seconds of no zooming
        scheduleHighResRender();
        
        return true;
    }

    // Function to schedule high-resolution rendering
    function scheduleHighResRender() {
        // Clear any existing timeout
        if (highResTimeout) {
            clearTimeout(highResTimeout);
        }
        
        // Set new timeout for 100ms
        highResTimeout = setTimeout(() => {
            // Calculate optimal render scale based on current zoom
            const optimalRenderScale = Math.min(Math.max(currentScale, 2.0), 8.0);
            
            // Debug logging
            console.log(`Zoom check - Current zoom: ${currentScale.toFixed(1)}x, Current render: ${currentRenderScale.toFixed(1)}x, Optimal render: ${optimalRenderScale.toFixed(1)}x`);
            
            // Re-render if there's a significant difference in either direction
            const scaleDifference = Math.abs(optimalRenderScale - currentRenderScale) / currentRenderScale;
            console.log(`Scale difference: ${(scaleDifference * 100).toFixed(1)}%`);
            
            if (scaleDifference > 0.25) { // Lowered threshold to 25%
                if (optimalRenderScale > currentRenderScale) {
                    console.log(`✅ Re-rendering at HIGHER quality: ${optimalRenderScale}x (was ${currentRenderScale}x)`);
                } else {
                    console.log(`✅ Re-rendering at LOWER quality: ${optimalRenderScale}x (was ${currentRenderScale}x)`);
                }
                renderCurrentPage(true, optimalRenderScale);
            } else {
                console.log(`❌ No re-render needed (difference ${(scaleDifference * 100).toFixed(1)}% < 25%)`);
            }
        }, 100);
    }

    // Zoom handlers
    zoomOutButton.addEventListener('click', () => {
        if (currentScale > 0.5) {
            applyZoom(Math.max(0.5, currentScale * 0.8));
        }
    });

    zoomInButton.addEventListener('click', () => {
        if (currentScale < 40.0) {
            applyZoom(Math.min(40.0, currentScale * 1.25));
        }
    });

    // Panning functionality
    function updateCursorStyle() {
        if (currentScale > 1.0) {
            canvas.style.cursor = isPanning ? 'grabbing' : 'grab';
        } else {
            canvas.style.cursor = 'default';
        }
    }

    canvas.addEventListener('mousedown', (e) => {
        if (currentScale > 1.0) {
            isPanning = true;
            lastMouseX = e.clientX;
            lastMouseY = e.clientY;
            updateCursorStyle();
        }
    });

    canvas.addEventListener('mousemove', (e) => {
        if (isPanning) {
            const deltaX = e.clientX - lastMouseX;
            const deltaY = e.clientY - lastMouseY;

            imageX += deltaX;
            imageY += deltaY;

            lastMouseX = e.clientX;
            lastMouseY = e.clientY;

            renderCurrentPage(false); // Don't reload the image, just reapply pan
        } else {
            // Update cursor style based on zoom level
            updateCursorStyle();
        }
    });

    canvas.addEventListener('mouseup', () => {
        isPanning = false;
        updateCursorStyle();
    });

    canvas.addEventListener('mouseleave', () => {
        isPanning = false;
        updateCursorStyle();
    });

    // Mouse wheel for zooming
    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();

        if (e.deltaY < 0 && currentScale < 40.0) {
            // Zoom in
            applyZoom(Math.min(40.0, currentScale * 1.25));
        } else if (e.deltaY > 0 && currentScale > 0.5) {
            // Zoom out
            applyZoom(Math.max(0.5, currentScale * 0.8));
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
                if (currentScale < 40.0) {
                    applyZoom(Math.min(40.0, currentScale * 1.25));
                }
                break;
            case '-':
                if (currentScale > 0.5) {
                    applyZoom(Math.max(0.5, currentScale * 0.8));
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

            // Cache the PDF data in localStorage
            try {
                const base64Data = btoa(String.fromCharCode(...uint8Array));
                localStorage.setItem('cachedPDF', base64Data);
                localStorage.setItem('cachedPDFName', file.name);
            } catch (e) {
                console.warn('Failed to cache PDF (too large for localStorage):', e);
                // Clear any partial data
                localStorage.removeItem('cachedPDF');
                localStorage.removeItem('cachedPDFName');
            }

            await loadPDFData(uint8Array);

        } catch (error) {
            console.error('Error loading PDF:', error);
            alert('Error loading PDF: ' + error.message);
        }
    }

    // Function to load PDF data (used by both file loading and cache loading)
    async function loadPDFData(uint8Array) {
        pdfViewer = new PdfViewer();
        await pdfViewer.load_pdf(uint8Array);

        totalPages = pdfViewer.get_page_count();
        currentPage = 1;
        currentScale = 2.0;
        imageX = 0;
        imageY = 0;
        currentRenderScale = 2.0; // Reset render scale

        fileSelector.style.display = 'none';
        viewer.style.display = 'flex';

        // Set max attribute for page input
        pageInput.setAttribute('max', totalPages);
        pageInput.value = currentPage;

        updateZoomInfo();
        renderCurrentPage();
    }

    // Function to load cached PDF on startup
    async function loadCachedPDF() {
        try {
            const cachedData = localStorage.getItem('cachedPDF');
            const cachedName = localStorage.getItem('cachedPDFName');
            
            if (cachedData) {
                console.log('Loading cached PDF:', cachedName || 'Unknown');
                
                // Convert base64 back to Uint8Array
                const binaryString = atob(cachedData);
                const uint8Array = new Uint8Array(binaryString.length);
                for (let i = 0; i < binaryString.length; i++) {
                    uint8Array[i] = binaryString.charCodeAt(i);
                }
                
                await loadPDFData(uint8Array);
            }
        } catch (error) {
            console.warn('Failed to load cached PDF:', error);
            // Clear corrupted cache data
            localStorage.removeItem('cachedPDF');
            localStorage.removeItem('cachedPDFName');
        }
    }

    // Store the current image for panning without reloading
    let currentImage = null;

    async function renderCurrentPage(reloadImage = true, renderScale = null) {
        if (!pdfViewer) return;

        try {
            if (reloadImage) {
                // Show loading indicator and start timer
                showLoading();
                startRenderTimer();
                
                // Reset image position when changing pages
                imageX = 0;
                imageY = 0;

                // Use provided render scale or calculate optimal one
                if (renderScale === null) {
                    // For page changes, use a reasonable default based on current zoom
                    renderScale = Math.min(Math.max(currentScale, 2.0), 4.0);
                }
                
                currentRenderScale = renderScale;

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

                console.log(`Rendering page at ${renderScale}x quality`);
                const imageData = await pdfViewer.render_current_page(renderScale);

                const blob = new Blob([imageData], { type: 'image/png' });
                const url = URL.createObjectURL(blob);

                // Load the image and store it for later use
                currentImage = new Image();
                currentImage.onload = () => {
                    URL.revokeObjectURL(url);
                    drawCurrentImage();
                    updateCursorStyle();
                    hideLoading(); // Hide loading when image is ready
                    updateRenderTime(); // Show render time
                };
                currentImage.onerror = () => {
                    hideLoading(); // Hide loading on error too
                };
                currentImage.src = url;
            } else {
                // Just redraw the current image with the updated position
                if (currentImage) {
                    drawCurrentImage();
                }
            }

            function drawCurrentImage() {
                const ctx = canvas.getContext('2d');

                // Calculate the scaled dimensions
                const scaleFactor = currentScale / currentRenderScale;
                const scaledWidth = currentImage.width * scaleFactor;
                const scaledHeight = currentImage.height * scaleFactor;

                // Use viewport dimensions for canvas
                const canvasWidth = window.innerWidth;
                const canvasHeight = window.innerHeight;

                // Set canvas size
                canvas.width = canvasWidth;
                canvas.height = canvasHeight;

                // Clear the canvas
                ctx.clearRect(0, 0, canvas.width, canvas.height);

                // Calculate image position (center + user pan offset)
                const imageCenterX = canvasWidth / 2 + imageX;
                const imageCenterY = canvasHeight / 2 + imageY;
                const imageLeft = imageCenterX - scaledWidth / 2;
                const imageTop = imageCenterY - scaledHeight / 2;

                // Draw the image
                ctx.drawImage(currentImage, imageLeft, imageTop, scaledWidth, scaledHeight);
            }

            updatePageInfo();
            updateNavigationButtons();

        } catch (error) {
            hideLoading(); // Hide loading on error
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
        zoomInButton.disabled = currentScale >= 40.0;
    }

    function resetToFileSelection() {
        viewer.style.display = 'none';
        fileSelector.style.display = 'flex';
        pdfViewer = null;
        currentPage = 1;
        totalPages = 0;
        currentScale = 2.0;
        imageX = 0;
        imageY = 0;
        isPanning = false;
        currentImage = null;
        fileInput.value = '';
        currentRenderScale = 2.0;
        
        // Clear any pending high-res render
        if (highResTimeout) {
            clearTimeout(highResTimeout);
            highResTimeout = null;
        }
        
        // Clear cached PDF when explicitly going back to file selection
        localStorage.removeItem('cachedPDF');
        localStorage.removeItem('cachedPDFName');
    }
}

run().catch(console.error); 
