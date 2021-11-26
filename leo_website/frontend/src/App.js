import logo from './logo.svg';
import { GlobalStyle } from './GlobalStyle';

function App() {
  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/App.js</code> and save to reload.
        </p>
        <a
          className="App-link"
          href="/login"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React 2
        </a>
      </header>
      <GlobalStyle />
    </div>
  );
}

export default App;
