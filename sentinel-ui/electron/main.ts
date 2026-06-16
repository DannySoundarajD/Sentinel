import { app, BrowserWindow, globalShortcut, Tray, Menu, nativeImage, ipcMain } from 'electron';
import * as path from 'path';
import { fileURLToPath } from 'url';
import * as fs from 'fs';
import * as os from 'os';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow: BrowserWindow | null = null;
let tray: Tray | null = null;

const isDev = process.env.NODE_ENV !== 'production' && !app.isPackaged;

function getDaemonPort(): number {
  try {
    const home = os.homedir();
    const portPath = path.join(home, '.local/share/sentinx/sentinel/daemon.port');
    if (fs.existsSync(portPath)) {
      return parseInt(fs.readFileSync(portPath, 'utf-8').trim(), 10) || 8888;
    }
  } catch (e) {
    console.error('Failed to read daemon port:', e);
  }
  return 8888;
}

function createWindow() {
  if (mainWindow) {
    return;
  }

  mainWindow = new BrowserWindow({
    width: 1000,
    height: 700,
    frame: false,
    transparent: false,
    backgroundColor: '#1e1e2e',
    alwaysOnTop: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      nodeIntegration: false,
      contextIsolation: true,
    },
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  ipcMain.on('window-minimize', () => {
    mainWindow?.minimize();
  });

  ipcMain.on('window-maximize', () => {
    if (mainWindow) {
      if (mainWindow.isMaximized()) {
        mainWindow.unmaximize();
      } else {
        mainWindow.maximize();
      }
    }
  });

  ipcMain.on('window-close', () => {
    mainWindow?.close();
  });

  ipcMain.on('get-daemon-port', (event) => {
    event.returnValue = getDaemonPort();
  });

  if (isDev) {
    let port = '51793';
    try {
      const portFilePath = path.join(__dirname, '../.port');
      if (fs.existsSync(portFilePath)) {
        port = fs.readFileSync(portFilePath, 'utf-8').trim();
      }
    } catch (e) {
      console.error('Failed to read Vite port file:', e);
    }
    mainWindow.loadURL(`http://localhost:${port}`);
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }

  // Hide window when it loses focus (optional, usually Spotlight behavior)
  // mainWindow.on('blur', () => mainWindow?.hide());
}

function toggleWindow() {
  if (!mainWindow) {
    createWindow();
  } else {
    if (mainWindow.isVisible()) {
      mainWindow.hide();
    } else {
      mainWindow.show();
      mainWindow.focus();
    }
  }
}

function createTray() {
  // You should provide a real icon in production
  const icon = nativeImage.createEmpty();
  tray = new Tray(icon);
  const contextMenu = Menu.buildFromTemplate([
    { label: 'Open Sentinel', click: () => { mainWindow?.show(); mainWindow?.focus(); } },
    { type: 'separator' },
    { label: 'Quit', click: () => { app.quit(); } }
  ]);
  tray.setToolTip('Sentinel AI');
  tray.setContextMenu(contextMenu);
}

const gotTheLock = app.requestSingleInstanceLock();

if (!gotTheLock) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (mainWindow) {
      if (mainWindow.isVisible()) {
        mainWindow.hide();
      } else {
        if (mainWindow.isMinimized()) mainWindow.restore();
        mainWindow.show();
        mainWindow.focus();
      }
    }
  });

  app.whenReady().then(() => {
    createWindow();
    createTray();

    // Register Global Shortcut: Super+Space
    const ret = globalShortcut.register('Super+Space', () => {
      toggleWindow();
    });

    if (!ret) {
      console.error('Registration failed for global shortcut Super+Space');
    }

    app.on('activate', () => {
      if (BrowserWindow.getAllWindows().length === 0) {
        createWindow();
      }
    });
  });
}

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('will-quit', () => {
  globalShortcut.unregisterAll();
});
