"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const symbols_1 = require("./symbols");
function activate(context) {
    console.log('LPPO Language Support is now active');
    // Register document formatting provider
    const formattingProvider = vscode.languages.registerDocumentFormattingEditProvider('lppo', {
        async provideDocumentFormattingEdits(document) {
            const formatterPath = path.join(context.extensionPath, 'formatters', 'out', 'lppo-format.js');
            if (!fs.existsSync(formatterPath)) {
                // Fallback to built-in simple formatting
                return provideSimpleFormatting(document);
            }
            // Use external formatter
            return new Promise((resolve, reject) => {
                const { spawn } = require('child_process');
                const proc = spawn('node', [formatterPath, '--stdin'], {
                    cwd: path.dirname(formatterPath)
                });
                let stdout = '';
                let stderr = '';
                proc.stdin.write(document.getText());
                proc.stdin.end();
                proc.stdout.on('data', (data) => {
                    stdout += data.toString();
                });
                proc.stderr.on('data', (data) => {
                    stderr += data.toString();
                });
                proc.on('close', (code) => {
                    if (code !== 0) {
                        console.error('Formatter error:', stderr);
                        resolve(provideSimpleFormatting(document));
                        return;
                    }
                    const fullRange = new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length));
                    resolve([vscode.TextEdit.replace(fullRange, stdout)]);
                });
            });
        }
    });
    context.subscriptions.push(formattingProvider);
    // Register symbol providers: document outline, go-to-definition,
    // find references, and hover info for classes, structs, enums, traits,
    // modules, impls, procedures, methods, fields, and enum variants.
    const documentSymbolProvider = vscode.languages.registerDocumentSymbolProvider('lppo', {
        provideDocumentSymbols(document) {
            return (0, symbols_1.parseSymbols)(document.getText()).map(symbols_1.toDocumentSymbol);
        },
    });
    context.subscriptions.push(documentSymbolProvider);
    const definitionProvider = vscode.languages.registerDefinitionProvider('lppo', {
        provideDefinition(document, position) {
            const word = (0, symbols_1.getWordAt)(document, position);
            if (!word) {
                return undefined;
            }
            const symbols = (0, symbols_1.flattenSymbols)((0, symbols_1.parseSymbols)(document.getText()));
            const definition = symbols.find((symbol) => (0, symbols_1.rangeContains)(symbol.selectionRange, position));
            const target = definition ??
                symbols.find((symbol) => symbol.name === word && (0, symbols_1.rangeContains)(symbol.range, position)) ??
                symbols.find((symbol) => symbol.name === word);
            if (!target) {
                return undefined;
            }
            return new vscode.Location(document.uri, target.range);
        },
    });
    context.subscriptions.push(definitionProvider);
    const referenceProvider = vscode.languages.registerReferenceProvider('lppo', {
        provideReferences(document, position, context) {
            const word = (0, symbols_1.getWordAt)(document, position);
            if (!word) {
                return [];
            }
            const symbols = (0, symbols_1.flattenSymbols)((0, symbols_1.parseSymbols)(document.getText()));
            const definition = symbols.find((symbol) => (0, symbols_1.rangeContains)(symbol.selectionRange, position));
            const target = definition ?? symbols.find((symbol) => symbol.name === word);
            if (!target) {
                return [];
            }
            const regex = new RegExp(`\\b${(0, symbols_1.escapeRegExp)(target.name)}\\b`, 'g');
            const text = document.getText();
            const locations = [];
            let match;
            while ((match = regex.exec(text)) !== null) {
                const start = document.positionAt(match.index);
                const end = document.positionAt(match.index + target.name.length);
                if (!context.includeDeclaration &&
                    definition &&
                    start.isEqual(definition.selectionRange.start) &&
                    end.isEqual(definition.selectionRange.end)) {
                    continue;
                }
                locations.push(new vscode.Location(document.uri, new vscode.Range(start, end)));
            }
            return locations;
        },
    });
    context.subscriptions.push(referenceProvider);
    const hoverProvider = vscode.languages.registerHoverProvider('lppo', {
        provideHover(document, position) {
            const word = (0, symbols_1.getWordAt)(document, position);
            if (!word) {
                return undefined;
            }
            const symbols = (0, symbols_1.flattenSymbols)((0, symbols_1.parseSymbols)(document.getText()));
            const target = symbols.find((symbol) => (0, symbols_1.rangeContains)(symbol.selectionRange, position)) ??
                symbols.find((symbol) => symbol.name === word);
            if (!target) {
                return undefined;
            }
            const markdown = new vscode.MarkdownString();
            markdown.appendMarkdown(`**${(0, symbols_1.kindLabel)(target.kind)}** \`${target.name}\``);
            if (target.containerName) {
                markdown.appendMarkdown(`\n\n*Container:* ${target.containerName}`);
            }
            if (target.detail) {
                markdown.appendMarkdown(`\n\n\`\`\`lppo\n${target.detail}\n\`\`\``);
            }
            return new vscode.Hover(markdown, target.range);
        },
    });
    context.subscriptions.push(hoverProvider);
    // Register document range formatting provider
    const rangeFormattingProvider = vscode.languages.registerDocumentRangeFormattingEditProvider('lppo', {
        async provideDocumentRangeFormattingEdits(document, range) {
            const text = document.getText(range);
            const formatted = formatText(text);
            return [vscode.TextEdit.replace(range, formatted)];
        }
    });
    context.subscriptions.push(rangeFormattingProvider);
    // Register "Run LPPO Script" command
    const runCommand = vscode.commands.registerCommand('lppo.run', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            void vscode.window.showWarningMessage('No active editor to run.');
            return;
        }
        const document = editor.document;
        if (document.languageId !== 'lppo') {
            void vscode.window.showWarningMessage('Active file is not an LPPO script.');
            return;
        }
        const filePath = document.uri.fsPath;
        const config = vscode.workspace.getConfiguration('lppo');
        const executablePath = config.get('executablePath', '');
        const runInTerminal = config.get('runInTerminal', true);
        if (!executablePath) {
            const openSettings = 'Open Settings';
            const choice = await vscode.window.showErrorMessage('LPPO executable path is not configured. Set "lppo.executablePath" in settings to the path of the built lppo binary.', openSettings);
            if (choice === openSettings) {
                await vscode.commands.executeCommand('workbench.action.openSettings', '@ext:lppo.lppo');
            }
            return;
        }
        if (!fs.existsSync(executablePath)) {
            void vscode.window.showErrorMessage(`LPPO executable not found at: ${executablePath}\nCheck the "lppo.executablePath" setting.`);
            return;
        }
        const workingDir = path.dirname(filePath);
        if (runInTerminal) {
            const terminal = vscode.window.createTerminal({
                name: 'LPPO',
                cwd: workingDir
            });
            terminal.sendText(`"${executablePath}" "${filePath}"`);
            terminal.show();
        }
        else {
            const outputChannel = vscode.window.createOutputChannel('LPPO');
            outputChannel.clear();
            outputChannel.show();
            const { spawn } = require('child_process');
            const proc = spawn(executablePath, [filePath], {
                cwd: workingDir
            });
            let stdout = '';
            let stderr = '';
            proc.stdout.on('data', (data) => {
                stdout += data.toString();
                outputChannel.append(data.toString());
            });
            proc.stderr.on('data', (data) => {
                stderr += data.toString();
                outputChannel.append(data.toString());
            });
            proc.on('close', (code) => {
                outputChannel.appendLine(`\n--- Process exited with code ${code} ---`);
            });
            proc.on('error', (err) => {
                outputChannel.appendLine(`\n--- Error: ${err.message} ---`);
                void vscode.window.showErrorMessage(`Failed to run LPPO script: ${err.message}`);
            });
        }
    });
    context.subscriptions.push(runCommand);
}
function provideSimpleFormatting(document) {
    const text = document.getText();
    const formatted = formatText(text);
    if (formatted === text) {
        return [];
    }
    const fullRange = new vscode.Range(document.positionAt(0), document.positionAt(text.length));
    return [vscode.TextEdit.replace(fullRange, formatted)];
}
function formatText(text) {
    const lines = text.split('\n');
    const output = [];
    let indentLevel = 0;
    const indentSize = 4;
    const indentStr = ' '.repeat(indentSize);
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const trimmed = line.trim();
        if (trimmed === '' || trimmed.startsWith('//') || trimmed.startsWith('/*')) {
            output.push(line);
            continue;
        }
        // Decrease indent for closing braces and block keywords
        if (trimmed.startsWith('}') || trimmed === 'end' || trimmed === 'endseg' ||
            trimmed.startsWith('else') || trimmed.startsWith('elif') ||
            trimmed.startsWith('catch') || trimmed.startsWith('finally') ||
            trimmed.startsWith('case') || trimmed.startsWith('default')) {
            indentLevel = Math.max(0, indentLevel - 1);
        }
        const indent = indentStr.repeat(indentLevel);
        output.push(indent + trimmed);
        // Increase indent for opening braces
        const openBraces = (trimmed.match(/{/g) || []).length;
        const closeBraces = (trimmed.match(/}/g) || []).length;
        indentLevel += openBraces - closeBraces;
        // Handle block start keywords
        if (trimmed.endsWith('{') && !trimmed.endsWith('{}')) {
            // Already counted in openBraces
        }
    }
    return output.join('\n');
}
function deactivate() {
    console.log('LPPO Language Support deactivated');
}
//# sourceMappingURL=extension.js.map