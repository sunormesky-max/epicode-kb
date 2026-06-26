import { BrowserRouter, Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import Home from './pages/Home'
import Upload from './pages/Upload'
import Search from './pages/Search'
import Review from './pages/Review'
import Graph from './pages/Graph'
import Login from './pages/Login'
import AdminConsole from './pages/AdminConsole'
import SpaceSettings from './pages/SpaceSettings'
import MemoryEditor from './pages/MemoryEditor'
import AgentIntegration from './pages/AgentIntegration'
import HealthDashboard from './pages/HealthDashboard'
import ConflictCenter from './pages/ConflictCenter'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route path="/" element={<Home />} />
          <Route path="/upload" element={<Upload />} />
          <Route path="/search" element={<Search />} />
          <Route path="/review" element={<Review />} />
          <Route path="/conflicts" element={<ConflictCenter />} />
          <Route path="/health" element={<HealthDashboard />} />
          <Route path="/graph" element={<Graph />} />
          <Route path="/spaces/:id/settings" element={<SpaceSettings />} />
          <Route path="/memories/:id/edit" element={<MemoryEditor />} />
          <Route path="/agent" element={<AgentIntegration />} />
        </Route>
        <Route path="/login" element={<Login />} />
      </Routes>
    </BrowserRouter>
  )
}
