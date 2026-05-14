# Formal Functional Test Plan: Mission Control Frontend

**Reference Specification:** `MissionControl_Frontend_Specification.md`
**System Under Test (SUT):** `rusty-SUNDIALS` Mission Control React Application
**Scope:** Client-side routing, API interception/mocking, component rendering, and role-based access.

---

## 1. Routing & Navigation Tests

### Test 1.1: Default Route Initialization
*   **Requirement:** Accessing the root URL (`/`) must load the Dashboard/Terminal interface.
*   **Action:** Navigate to `http://localhost:5173/`.
*   **Expected Result:** The `TerminalPage` component renders. The sidebar shows "Dashboard" as the active tab.

### Test 1.2: Deep Linking and Page Navigation
*   **Requirement:** Users can navigate between all primary hubs via the Sidebar without a full page reload, and direct URL access (deep linking) resolves correctly.
*   **Action:** 
    1. Click "Discoveries" -> URL changes to `/discoveries`.
    2. Click "Education" -> URL changes to `/education`.
    3. Reload the page at `/publications`.
*   **Expected Result:** The respective pages render. The reload at `/publications` must successfully render the `PublicationsPage` without returning a 404 error (verifying the Nginx `try_files` configuration).

---

## 2. API Interception & Mock Data Tests

### Test 2.1: Discoveries Payload Interception
*   **Requirement:** The application must intercept `/api/discoveries` and return static mock data when the backend is disconnected (Serverless Demo Mode).
*   **Action:** Load the `/discoveries` route. Monitor the network tab.
*   **Expected Result:** No outbound HTTP request to a live backend should fail. The `MOCK_RESULTS` object is parsed, and Phase I-IV and ITER Phase III data cards (e.g., Protocol F: Tensor-Train) are visibly rendered in the grid.

### Test 2.2: Publications Report Fetching
*   **Requirement:** The `/api/report` endpoint must be mocked and successfully return `MOCK_REPORT`.
*   **Action:** Load the `/publications` route.
*   **Expected Result:** The system bypasses the "NO REPORT GENERATED" state and immediately renders the "Disruptive Physics & Autonomous AI" paper abstract, equations, and LaTeX sections.

---

## 3. Component Functional Tests

### Test 3.1: GlowPanel Container Rendering
*   **Requirement:** The `GlowPanel` must correctly apply neon styling and render child elements.
*   **Action:** Inspect any rendered GlowPanel (e.g., the "ITER FUSION" panel in the Education tab).
*   **Expected Result:** The panel displays the provided `title` prop. CSS variables for border color and neon box-shadow are applied when hovered.

### Test 3.2: Education Media Hub Tab Switching
*   **Requirement:** The Education page must seamlessly toggle between its internal states (Infographics, ITER Fusion, Animations, Articles).
*   **Action:** Navigate to `/education`. Click the "ITER Fusion Master" button, then click "Infographics".
*   **Expected Result:** State updates correctly. When "ITER Fusion" is clicked, the 8K plasma core hero image (`/iter_fusion_hero.png`) and Phase III protocols are displayed. When "Infographics" is clicked, the Protocol K and M images are shown.

### Test 3.3: Media Kit Download Action
*   **Requirement:** Clicking "DOWNLOAD FULL MEDIA KIT" should trigger the download action.
*   **Action:** On the `/education` page, click the download button.
*   **Expected Result:** A browser alert triggers indicating "Downloading Full Media Kit" (or the file download initiates if linked to a static `.zip`).

---

## 4. Role-Based Access Control (RBAC) Tests

### Test 4.1: Unauthorized Run Execution
*   **Requirement:** Users without the `admin` role cannot trigger live autonomous research cycles.
*   **Action:** Using a standard `guest` or `viewer` token, navigate to the Dashboard and attempt to click the "INITIALIZE AUTORESEARCH" button.
*   **Expected Result:** The button is either disabled or triggers a "Permission Denied" warning. No `/api/run` request is dispatched.

### Test 4.2: Admin Authorization
*   **Requirement:** Admin users can trigger live runs.
*   **Action:** Authenticate with `role === 'admin'`. Click the "INITIALIZE" button.
*   **Expected Result:** The terminal simulator begins emitting text streams, simulating or triggering an active Python GPU execution.

---

## 5. Security & Legal Verification

### Test 5.1: Copyright Tagging
*   **Requirement:** The UI must display the proper legal attribution.
*   **Action:** Inspect the footer of the application and the `PublicationsPage` report header.
*   **Expected Result:** The text "SocrateAI Lab" and "Xavier Callens" is visibly rendered as the copyright holder/author.
