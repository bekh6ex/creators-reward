Feature: Viewer registration and login
  As a new viewer
  I want to create an account and later log back in
  So that I can access my dashboard across sessions

  Scenario: A new viewer registers and logs back in with the same credentials
    Given the visitor is on the registration page
    When the visitor registers a new account
    And the visitor logs out
    And the visitor logs in with the same credentials
    Then the viewer dashboard is shown
