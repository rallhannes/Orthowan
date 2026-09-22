const { invoke } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;
const { open, save } = window.__TAURI__.dialog;

const dropZone = document.getElementById('drop-zone');
const btnConvert = document.getElementById('btn-convert');
const btnSave = document.getElementById('btn-save');
const fileListContainer = document.getElementById('file-list-container');

let pendingFiles = [];
let convertedFiles = [];

// Tauri Native Drag & Drop
listen('tauri://file-drop', event => {
    dropZone.classList.remove('dragover');
    const paths = event.payload.filter(p => p.toLowerCase().endsWith('.ifc'));
    addFiles(paths);
});

listen('tauri://file-drop-hover', event => {
    dropZone.classList.add('dragover');
});

listen('tauri://file-drop-cancelled', event => {
    dropZone.classList.remove('dragover');
});

dropZone.addEventListener('click', async () => {
    const selected = await open({
        multiple: true,
        filters: [{ name: 'IFC Files', extensions: ['ifc'] }]
    });
    if (Array.isArray(selected)) {
        addFiles(selected);
    } else if (selected) {
        addFiles([selected]);
    }
});

function addFiles(paths) {
    if (!paths || paths.length === 0) return;
    for (const path of paths) {
        if (!pendingFiles.includes(path)) {
            pendingFiles.push(path);
        }
    }
    updateUI();
}

function updateUI() {
    btnConvert.disabled = pendingFiles.length === 0;
    
    fileListContainer.innerHTML = '';
    
    // Show pending files
    for (const file of pendingFiles) {
        const div = document.createElement('div');
        div.className = 'file-item';
        const name = file.split(/[\\/]/).pop();
        div.innerHTML = `<span style="flex: 1;">${name}</span> <span class="status-icon status-pending">⏳</span>`;
        fileListContainer.appendChild(div);
    }
    
    // Show converted files
    for (let i = 0; i < convertedFiles.length; i++) {
        const cf = convertedFiles[i];
        const div = document.createElement('div');
        div.className = 'file-item';
        
        const input = document.createElement('input');
        input.type = 'text';
        input.value = cf.newName;
        input.addEventListener('change', (e) => {
            convertedFiles[i].newName = e.target.value;
        });
        
        const status = document.createElement('span');
        status.className = 'status-icon ' + (cf.success ? 'status-success' : 'status-error');
        status.textContent = cf.success ? '✅' : '❌';
        status.title = cf.message || '';
        
        div.appendChild(input);
        div.appendChild(status);
        fileListContainer.appendChild(div);
    }
    
    btnSave.disabled = convertedFiles.filter(cf => cf.success).length === 0;
}

btnConvert.addEventListener('click', async () => {
    btnConvert.disabled = true;
    const filesToConvert = [...pendingFiles];
    pendingFiles = [];
    
    for (const file of filesToConvert) {
        try {
            const result = await invoke('convert_file', { path: file });
            convertedFiles.push({
                originalPath: file,
                tempPath: result.temp_path,
                newName: result.suggested_name,
                success: true,
                message: `Repariert: ${result.repaired}`
            });
        } catch (err) {
            convertedFiles.push({
                originalPath: file,
                tempPath: "",
                newName: file.split(/[\\/]/).pop() + "_error.ifc",
                success: false,
                message: err
            });
        }
    }
    updateUI();
});

btnSave.addEventListener('click', async () => {
    const successFiles = convertedFiles.filter(cf => cf.success);
    if (successFiles.length === 0) return;
    
    const outDir = await open({
        directory: true,
        multiple: false
    });
    
    if (outDir) {
        try {
            await invoke('save_files', {
                files: successFiles.map(cf => ({
                    temp_path: cf.tempPath,
                    new_name: cf.newName
                })),
                outDir: Array.isArray(outDir) ? outDir[0] : outDir
            });
            alert('Dateien erfolgreich gespeichert!');
            convertedFiles = [];
            updateUI();
        } catch (err) {
            alert('Fehler beim Speichern: ' + err);
        }
    }
});
