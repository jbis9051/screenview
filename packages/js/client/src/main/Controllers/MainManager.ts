import MainWindow from '../ViewModel/MainWindow';

export default class MainManager {
    window: MainWindow | null = null;

    focus() {
        if (!this.window) {
            this.window = new MainWindow();
            this.window.on('close', () => {
                this.window = null;
            });
        } else {
            this.window.focus();
        }
    }
}
