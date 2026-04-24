import { Routes, Route } from 'react-router-dom';
import { Sidebar } from './components/Sidebar';
import { Player } from './components/Player';
import { Home } from './pages/Home';
import { Search } from './pages/Search';
import { Playlist } from './pages/Playlist';
import './styles/app.css';

function App() {
  return (
    <div className="app-shell">
      <Sidebar />
      <main className="app-main">
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/search" element={<Search />} />
          <Route path="/playlist/:id" element={<Playlist />} />
        </Routes>
      </main>
      <Player />
    </div>
  );
}

export default App;
