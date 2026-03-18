// Initialize Ace Editor
const editor = ace.edit("editor");
editor.setTheme("ace/theme/monokai");
editor.session.setMode("ace/mode/text");
editor.setShowPrintMargin(false);
editor.setOption("showLineNumbers", true); // Always display line numbers

// Load saved keybinding preference from localStorage
const savedKeybinding = localStorage.getItem('aceKeybinding') || 'default';
const keybindingSelect = document.getElementById('keybinding-select');
keybindingSelect.value = savedKeybinding;

// Apply the saved keybinding
if (savedKeybinding === 'vim') {
    editor.setKeyboardHandler('ace/keyboard/vim');
} else {
    editor.setKeyboardHandler(null); // Default bindings
}

// Draw line on canvas based on editor content
function drawLine() {
    const canvas = document.getElementById('canvas');
    const ctx = canvas.getContext('2d');

    // Set canvas size
    canvas.width = 600;
    canvas.height = 600;

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Get editor content
    const content = editor.getValue();
    const lines = content.split('\n');

    // Calculate x and y
    const x = lines.length;

    let maxColumn = 0;
    lines.forEach(line => {
        if (line.length > maxColumn) {
            maxColumn = line.length;
        }
    });
    const y = maxColumn;

    // Draw line from (0, 0) to (x, y)
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(x * 10, y * 10); // Scale for visibility
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw coordinate labels
    ctx.fillStyle = '#666';
    ctx.font = '12px monospace';
    ctx.fillText(`(0, 0)`, 5, 15);
    ctx.fillText(`(${x}, ${y})`, x * 10 + 5, y * 10 + 15);

    // Draw point at end
    ctx.beginPath();
    ctx.arc(x * 10, y * 10, 4, 0, 2 * Math.PI);
    ctx.fillStyle = '#ff0000';
    ctx.fill();
}

// Listen for changes in the editor
editor.session.on('change', function() {
    drawLine();
});

// Handle keybinding selection
keybindingSelect.addEventListener('change', function() {
    const selectedBinding = this.value;

    // Save to localStorage
    localStorage.setItem('aceKeybinding', selectedBinding);

    // Apply the keybinding
    if (selectedBinding === 'vim') {
        editor.setKeyboardHandler('ace/keyboard/vim');
    } else {
        editor.setKeyboardHandler(null); // Default bindings
    }
});

// Initial draw
drawLine();

