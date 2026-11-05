import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

export function activate(context: vscode.ExtensionContext) {
  console.log('LPPO Language Support is now active');

  // Register document formatting provider
  const formattingProvider = vscode.languages.registerDocumentFormattingEditProvider(
    'lppo',
    {
      async provideDocumentFormattingEdits(document: vscode.TextDocument): Promise<vscode.TextEdit[]> {
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

          proc.stdout.on('data', (data: Buffer) => {
            stdout += data.toString();
          });

          proc.stderr.on('data', (data: Buffer) => {
            stderr += data.toString();
          });

          proc.on('close', (code: number) => {
            if (code !== 0) {
              console.error('Formatter error:', stderr);
              resolve(provideSimpleFormatting(document));
              return;
            }

            const fullRange = new vscode.Range(
              document.positionAt(0),
              document.positionAt(document.getText().length)
            );
            resolve([vscode.TextEdit.replace(fullRange, stdout)]);
          });
        });
      }
    }
  );

  context.subscriptions.push(formattingProvider);

  // Register document range formatting provider
  const rangeFormattingProvider = vscode.languages.registerDocumentRangeFormattingEditProvider(
    'lppo',
    {
      async provideDocumentRangeFormattingEdits(
        document: vscode.TextDocument,
        range: vscode.Range
      ): Promise<vscode.TextEdit[]> {
        const text = document.getText(range);
        const formatted = formatText(text);
        return [vscode.TextEdit.replace(range, formatted)];
      }
    }
  );

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
    const executablePath = config.get<string>('executablePath', '');
    const runInTerminal = config.get<boolean>('runInTerminal', true);

    if (!executablePath) {
      const openSettings = 'Open Settings';
      const choice = await vscode.window.showErrorMessage(
        'LPPO executable path is not configured. Set "lppo.executablePath" in settings to the path of the built lppo binary.',
        openSettings
      );
      if (choice === openSettings) {
        await vscode.commands.executeCommand('workbench.action.openSettings', '@ext:lppo.lppo');
      }
      return;
    }

    if (!fs.existsSync(executablePath)) {
      void vscode.window.showErrorMessage(
        `LPPO executable not found at: ${executablePath}\nCheck the "lppo.executablePath" setting.`
      );
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
    } else {
      const outputChannel = vscode.window.createOutputChannel('LPPO');
      outputChannel.clear();
      outputChannel.show();

      const { spawn } = require('child_process');
      const proc = spawn(executablePath, [filePath], {
        cwd: workingDir
      });

      let stdout = '';
      let stderr = '';

      proc.stdout.on('data', (data: Buffer) => {
        stdout += data.toString();
        outputChannel.append(data.toString());
      });

      proc.stderr.on('data', (data: Buffer) => {
        stderr += data.toString();
        outputChannel.append(data.toString());
      });

      proc.on('close', (code: number | null) => {
        outputChannel.appendLine(`\n--- Process exited with code ${code} ---`);
      });

      proc.on('error', (err: Error) => {
        outputChannel.appendLine(`\n--- Error: ${err.message} ---`);
        void vscode.window.showErrorMessage(`Failed to run LPPO script: ${err.message}`);
      });
    }
  });

  context.subscriptions.push(runCommand);
}

function provideSimpleFormatting(document: vscode.TextDocument): vscode.TextEdit[] {
  const text = document.getText();
  const formatted = formatText(text);

  if (formatted === text) {
    return [];
  }

  const fullRange = new vscode.Range(
    document.positionAt(0),
    document.positionAt(text.length)
  );
  return [vscode.TextEdit.replace(fullRange, formatted)];
}

function formatText(text: string): string {
  const lines = text.split('\n');
  const output: string[] = [];
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

export function deactivate() {
  console.log('LPPO Language Support deactivated');
}
