import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { AuthProvider } from './hooks/useAuth';
import Sidebar from './components/Sidebar';
import Header from './components/Header';
import DashboardPage from './pages/DashboardPage';
import PipelinePage from './pages/PipelinePage';
import DiscoveriesPage from './pages/DiscoveriesPage';
import VerificationPage from './pages/VerificationPage';
import PhysicsPage from './pages/PhysicsPage';
import PublicationsPage from './pages/PublicationsPage';
import DocsPage from './pages/DocsPage';
import LeaderboardPage from './pages/LeaderboardPage';
import EducationPage from './pages/EducationPage';
import CostPage from './pages/CostPage';
import SettingsPage from './pages/SettingsPage';
import SopPage from './pages/SopPage';
import PeerReviewPage from './pages/PeerReviewPage';
import BenchmarksPage from './pages/BenchmarksPage';

export default function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <div className="app-layout">
          <Sidebar />
          <div className="main-area">
            <Header cost={46.62} budget={100} />
            <main className="page-content">
              <Routes>
                <Route path="/" element={<DashboardPage />} />
                <Route path="/pipeline" element={<PipelinePage />} />
                <Route path="/discoveries" element={<DiscoveriesPage />} />
                <Route path="/verification" element={<VerificationPage />} />
                <Route path="/physics" element={<PhysicsPage />} />
                <Route path="/publications" element={<PublicationsPage />} />
                <Route path="/education" element={<EducationPage />} />
                <Route path="/cost" element={<CostPage />} />
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/docs" element={<DocsPage />} />
                <Route path="/leaderboard" element={<LeaderboardPage />} />
                <Route path="/sop" element={<SopPage />} />
                <Route path="/peer-review" element={<PeerReviewPage />} />
                <Route path="/benchmarks" element={<BenchmarksPage />} />
              </Routes>
            </main>
          </div>
          <div className="scanline-overlay" />
        </div>
      </BrowserRouter>
    </AuthProvider>
  );
}
