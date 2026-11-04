import { createRoot } from 'react-dom/client';
import App from './App';
import { DiscardConfirmation } from './components/DiscardConfirmation';
createRoot(document.getElementById('root')!).render(<DiscardConfirmation><App /></DiscardConfirmation>);
