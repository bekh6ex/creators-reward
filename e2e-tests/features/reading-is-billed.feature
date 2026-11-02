@mode:serial @timeout:300_000
Feature: Reading is billed
  As a viewer
  I want what I read on a creator's website to decide my donation
  So that the creators I read are the ones I support

  Scenario: A creator connects their website
    Given a creator has registered in the creator portal
    When the creator connects the website "website1.crlocal"
    Then the creator's websites list "website1.crlocal" as "Verified"

  Scenario: A viewer's reading on the connected website is recorded for billing
    Given the key holders have published keys for the coming billing periods
    And a viewer has registered
    When the viewer enables donations on "website1.crlocal"
    And the viewer reads an article on "website1.crlocal"
    Then 2 page views are recorded for billing in the same billing period

  Scenario: The billing ceremony charges the viewer and pays the website
    Given the billing period of the viewer's page views has ended
    When the key holders run the billing ceremony for that period
    Then the viewer is charged "0.20 €"
    And "website1.crlocal" is paid "0.20 €"
